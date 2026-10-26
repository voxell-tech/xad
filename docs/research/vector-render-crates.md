# Survey: how existing crates/frameworks accelerate vector rendering and nearest-curve queries

Research notes, 2026-10-13. Companion to
[nearest-edge-acceleration.md](nearest-edge-acceleration.md), which concluded that the
literature answer for our problem (per-fragment nearest of ≤128 quadratic Beziers in
WGSL) is a coarse grid with conservative per-tile candidate lists. This document
surveys how *production code* — especially the Rust ecosystem — organizes the same
kind of work, and what is directly reusable.

Claims are cited to the projects' own repos/docs; anything not verified against a
reachable primary source is marked **uncertain**.

---

## 1. vello (linebender) — tile binning on an industrial scale

Vello today contains two renderer families
([ARCHITECTURE.md](https://github.com/linebender/vello/blob/main/ARCHITECTURE.md)):

- **Sparse Strips** (`vello_cpu`, `vello_gpu`): a CPU-side pipeline in `vello_common`
  where "paths are flattened, binned into tiles, and represented as sparse horizontal
  strips. Coverage data is retained where it is needed around path boundaries, while
  fully covered regions can be represented compactly." `vello_gpu` does this
  preprocessing on CPU, then uploads scheduled strips for GPU rasterization using
  ordinary vertex/fragment shaders (no compute required). Design background: the
  "Potato" hybrid CPU/GPU design doc and the Vello CPU master's thesis (both linked
  from ARCHITECTURE.md).
- **Compute-centric research renderer** (the published `vello` crate, now under
  `research/`): scene encoded into compact path/draw buffers; prefix-scan compute
  shaders parallelize the classically-sequential steps
  ([research README](https://github.com/linebender/vello/blob/main/research/vello_research/README.md)).
  The compute pipeline's historical stage names — flatten → binning → coarse
  (per-tile command list, "PTCL") → fine — come from the piet-gpu lineage; the WGSL
  sources live in `research/vello_shaders/shader` and the path encoding is described
  in `research/doc/pathseg.md`. **Uncertain**: the stage list above is from the
  project's historical design docs and blog posts (linked from
  `research/doc/blogs.md`), not re-verified from the current shader sources in this
  session.

Key point for xad: vello computes **coverage/winding, not distances**. Both families
share our target shape — a CPU (or coarse-GPU) pass that reduces "all segments" to
"segments relevant to this tile", followed by a per-tile consumer that loops a short
list. The fine stage reads a variable-length per-tile command list from a storage
buffer, which is exactly the buffer layout our conservative-candidate-list plan needs
(offsets + indices). Vello's full pipeline (sorting, clips, blends, prefix sums) is
massive overkill for 128 static edges, but its data layout is the pattern to copy.

## 2. lyon (lyon_geom / lyon_tessellation) — CPU tessellation + reusable curve math

`lyon_geom::QuadraticBezierSegment` provides, among others
([docs.rs](https://docs.rs/lyon_geom/latest/lyon_geom/struct.QuadraticBezierSegment.html)):

- `closest_point(pos) -> t` — "Analytic solution to finding the closest point on the
  curve" (the cubic solve we already do in WGSL, available on CPU for free);
- `distance_to_point`, `square_distance_to_point`;
- `bounding_box` (tight, via extrema) and `fast_bounding_box` ("conservative
  rectangle that contains the curve" — the control-polygon AABB our tile binning
  needs), plus `bounding_triangle` and `fat_line` (two conservative lines containing
  the segment — a tighter prune primitive than an AABB);
- `line_intersections`, splitting, monotonic decomposition, flattening with
  tolerance.

For fills, `lyon_tessellation` tessellates paths into triangle meshes on the CPU
(fixed-function rasterization; no per-pixel distance at all). That is an alternative
rendering strategy, not an acceleration of ours — but `lyon_geom` alone (small,
`euclid`-based, f32-capable via `to_f32`) could replace xad's hand-rolled CPU
polyline distance sampling for edit-time queries and for building per-tile lists.

## 3. kurbo (linebender) — exact nearest-point on CPU

`kurbo::ParamCurveNearest` defines `nearest(&self, p: Point, accuracy: f64) ->
Nearest`, implemented for `QuadBez`, `CubicBez`, `Line`, and `PathSeg`
([docs.rs](https://docs.rs/kurbo/latest/kurbo/trait.ParamCurveNearest.html); the
`QuadBez` impl is in `kurbo/src/quadbez.rs`). `Nearest` reports the squared distance
and the parameter `t`. kurbo also has `ParamCurveExtrema` for tight bounding boxes
and a robust cubic solver in `kurbo::common`. **Uncertain**: whether the `QuadBez`
`nearest` is a closed-form cubic solve or iterative subdivision was not verified from
source this session; either way the `accuracy` parameter makes it suitable for exact
CPU distances.

Fit for xad: kurbo is the most direct replacement for brute-force CPU polyline
sampling — exact per-edge distances for tile-bound computation, hit testing, and
validating the WGSL cubic solve against a reference. It is f64-only, which is fine
for preprocessing but means a conversion layer at the GPU boundary.

## 4. pathfinder (servo) — CPU tiling + GPU alpha-mask fills

Pathfinder 3 is a GPU rasterizer for fonts/vector graphics with two modes
([README](https://github.com/servo/pathfinder)): "D3D9" (hardware rasterization,
tiling done on CPU "using SIMD and Rayon") and "D3D11" (tiling moved to compute).
It performs "tile-based occlusion culling" and computes "exact fractional
trapezoidal area coverage on a per-pixel basis" — again coverage, not distance. The
CPU tiler clips each path into fixed-size tiles, classifies tiles as fully
inside/outside/boundary, and rasterizes only boundary tiles into alpha masks that the
GPU composites. **Uncertain**: the tile-classification detail is from Pathfinder's
design talks/wiki, not re-verified from `renderer/` sources this session.

Status: effectively in maintenance. Last substantive commits are dependency bumps
(most recent commit at time of writing: a `pathfinder_simd` bump, ~6 months ago); the
README still says "under heavy development and is incomplete", unchanged for years.
Useful as a design reference (CPU tiler feeding a dumb GPU stage — the same split we
want), not as a dependency.

## 5. One-paragraph surveys

- **forma (google, archived)**: "(thoroughly) parallelized experimental Rust
  vector-graphics renderer" with CPU and wgpu backends and an explicit 4-stage
  pipeline: Bezier curves → line segments → pixel segments → sorted pixel segments →
  painted tiles; it re-paints only tiles that changed
  ([README](https://github.com/google/forma)). Repository archived read-only
  2024-07-18 — a design reference only. Its sort-then-paint pixel-segment approach is
  a different decomposition from candidate lists and not applicable to distance
  queries, but the "update only changed tiles" idea matches xad's static-until-edit
  workload.
- **vger-rs (audulus)**: wgpu renderer that "draws one or more quads for each
  primitive and computes the actual primitive shape in the fragment function with an
  SDF"; crucially, "for path fills, vger splits paths into **horizontal slabs** (see
  `src/path.rs`) to reduce the number of tests in the fragment function", and its fill
  test is a reverse Loop–Blinn winding test rather than a quadratic solve
  ([README](https://github.com/audulus/vger-rs)). This is a shipping Rust/WGSL
  implementation of exactly the Slug-style banding idea the prior doc recommended —
  the closest existing-code analogue to our plan, worth reading `src/path.rs` before
  implementing.
- **femtovg**: Rust port of nanovg; "rendering is done via one OpenGL (ES) 3.0+
  backend" (plus a newer WGPU backend) ([README](https://github.com/femtovg/femtovg)).
  nanovg-lineage fills are CPU-tessellated fans/strips with stencil-based concave
  handling and shader-feathered AA (**uncertain** — inherited from nanovg's design,
  not re-verified in femtovg source). No per-pixel distance machinery; nothing to
  borrow for nearest-curve queries.
- **skia / skia-safe**: `skia-safe` is a binding to Google's C++ Skia. Skia's CPU
  raster backend uses scanline/analytic-coverage rasterization; its GPU backends
  (Ganesh, now Graphite) use a mix of tessellation, atlases, and analytic edge
  equations per draw (**uncertain** — summarized from skia.org architecture docs, not
  fetched this session). Skia solves fill coverage, not nearest-primitive; a heavy
  C++ dependency with nothing specific to borrow for this problem.

## 6. SDF-specific projects

- **msdfgen (Chlumský, C++)**: generates (M)SDF textures from shapes made of
  `LinearEdge`/`QuadraticEdge`/`CubicEdge` contours; runtime use is a texture sample
  + median-of-3 + smoothstep ([README](https://github.com/Chlumsky/msdfgen)). On
  distance assignment: the core generator evaluates, per output texel, the signed
  distance by iterating the shape's contours and edges and keeping the minimum —
  **no spatial acceleration structure**; the shapes are small (one glyph, tens of
  edges) so brute force per texel is fine. **Uncertain**: "no spatial structure" is
  inferred from the structure of `core/` (edge-segment classes with per-edge
  `signedDistance` methods, scanline-based sign resolution) and the project's
  glyph-scale focus; the core loop was not re-read line-by-line this session. Lesson:
  at n ≈ edge-count-of-a-glyph, production code doesn't bother accelerating — but
  msdfgen pays that cost once offline, whereas xad pays it per fragment per frame.
- **Rust ports**: `fdsm` is a Rust reimplementation of the msdfgen algorithm on
  `ttf-parser`, and the `msdf` crate provides bindings/port of msdfgen
  (**uncertain** — crate descriptions from crates.io metadata; neither fetched this
  session). Same algorithmic shape as msdfgen: per-texel brute-force edge iteration
  at bake time.
- **glyphon / GPU font stacks**: glyphon renders text in wgpu via a glyph atlas —
  glyphs are rasterized on CPU (`cosmic-text`/swash) and packed into a texture; the
  shader just samples the atlas (**uncertain** — from the project README, not fetched
  this session). I.e. the dominant Rust text stack avoids per-pixel curve math
  entirely; only Slug-style systems keep curves on the GPU.
- **Slug (Lengyel, C++, commercial)** recap: per-glyph precomputed horizontal/vertical
  *bands*, each storing a sorted shortlist of the curves intersecting the band; the
  fragment shader ray-tests only the band's list (coverage/winding, not distance).
  Sources: [JCGT 6(2) paper](https://jcgt.org/published/0006/02/02/),
  [i3D 2018 slides](https://terathon.com/i3d2018_lengyel.pdf). vger-rs's horizontal
  slabs are the open-source echo of this.

## 7. BVH / spatial crates — nearest-primitive queries to mirror in WGSL

- **parry2d (dimforge)**: the strongest match. Its `partitioning` module provides a
  `Bvh` ("Bounding Volume Hierarchy for spatial queries"), with a `BvhLeafCost` trait
  explicitly for **best-first traversal** ("Cost associated to a BVH leaf during
  best-first traversal") and a generic `traverse` controlled by `TraversalAction`
  ([docs.rs partitioning](https://docs.rs/parry2d/latest/parry2d/partitioning/index.html)).
  (Older parry versions called this structure `Qbvh` — a quantized, SIMD-friendly
  4-wide tree; current docs.rs 0.31 exposes `Bvh`/`BvhNodeWide`, a 2-wide node
  layout.) Point queries come from the `PointQuery` trait — `project_point`,
  `distance_to_point`, `project_local_point_with_max_dist` (early-out beyond a max
  distance), implemented for `Polyline`, `TriMesh`, `Compound`, etc.
  ([docs.rs PointQuery](https://docs.rs/parry2d/latest/parry2d/query/point/trait.PointQuery.html)).
  `Polyline::project_local_point` is precisely "nearest point on a flattened curve
  set via branch-and-bound over a BVH" — a ready-made CPU oracle, and the
  `BvhNodeWide` 2-wide flattened node layout is a sensible template if a WGSL BVH is
  ever needed. Parry has no Bezier shape; curves must be flattened first.
- **rstar**: R*-tree with `nearest_neighbor` / `nearest_neighbor_iter` doing standard
  branch-and-bound over inserted envelopes (AABBs); arbitrary payloads via
  `GeomWithData` (**uncertain** — well-known crate API, docs.rs not fetched this
  session). Works for "nearest edge AABB", but gives bounds on boxes, not curves —
  the final exact distance test is still ours. For n=128 a flat array scan of AABBs
  beats tree overhead anyway.
- **bvh (crate `bvh`)**: ray-intersection-oriented BVH (built for ray tracing,
  `nalgebra`-based); it is not designed around nearest-point queries (**uncertain**
  — from crate documentation summary, not fetched this session). Not a fit.

## 8. Comparison

| Project | Strategy | Distance or coverage? | Acceleration structure | Reusable for xad? |
|---|---|---|---|---|
| vello (sparse strips + compute) | flatten → bin to tiles/strips → per-tile consume | Coverage/winding | Per-tile segment/command lists in storage buffers | Pattern: buffer layout for per-tile variable-length lists |
| lyon | CPU tessellation to triangles | Neither (mesh) | None needed | `lyon_geom`: analytic `closest_point`, conservative + tight AABBs, `fat_line` |
| kurbo | curve math library | Exact nearest on CPU | None | `ParamCurveNearest::nearest` replaces polyline sampling |
| pathfinder | CPU tiler → GPU alpha masks | Coverage (exact trapezoid area) | Per-tile classification + mask tiles | Design reference only; maintenance mode |
| forma (archived) | curves → pixel segments → sort → paint tiles | Coverage | Sorted pixel segments; per-tile damage tracking | "Repaint only changed tiles" idea |
| vger-rs | per-primitive quad + SDF in fragment shader | SDF for strokes; winding for fills | **Horizontal slabs per path** (`src/path.rs`) | Closest shipping analogue of our banding plan, in WGSL |
| femtovg | nanovg-style tessellation + stencil | Coverage | None | Nothing for this problem |
| skia (skia-safe) | scanline CPU / tess+atlas GPU | Coverage | Internal, not exposed | Nothing practical to borrow |
| msdfgen / fdsm / msdf | offline per-texel brute force over edges | Exact signed distance (baked) | **None** (glyph-scale n) | Confirms brute force is fine *offline*; median-of-3 trick if we ever bake |
| Slug | per-glyph band shortlists, curves in texture | Coverage (ray-curve winding) | Horizontal/vertical bands | The banding blueprint (commercial, C++) |
| parry2d | collision/geometry queries | Exact distance/projection | `Bvh` with best-first traversal (`BvhLeafCost`), 2-wide nodes | CPU nearest oracle on flattened curves; node layout template |
| rstar | R*-tree | AABB-level bounds | branch-and-bound NN | Marginal at n=128 |
| bvh crate | ray-tracing BVH | n/a | ray-oriented | Not a fit |

## 9. Takeaways for xad (≤128 quadratic Beziers, static until edit, WGSL fragment shader, bevy storage buffer)

What to copy:

1. **The tile/band shortlist is universal.** Every GPU renderer that keeps curves on
   the GPU — vello (tiles/strips), pathfinder (tiles), Slug (bands), vger-rs
   (horizontal slabs) — reduces "all segments" to a short per-region list before the
   per-pixel loop. None of them computes Voronoi cells or per-edge winning regions.
   This independently confirms the prior doc's recommendation (b): CPU-built
   conservative per-tile candidate lists.
2. **Read vger-rs `src/path.rs` before implementing.** It is a small, MIT-licensed,
   Rust+WGSL implementation of per-path horizontal slabs feeding a fragment-shader
   curve loop — the closest existing code to what we are about to write, at a
   comparable scale (UI paths, not SVG cities).
3. **Use kurbo (or lyon_geom) for the CPU side.** `kurbo::QuadBez::nearest` /
   `lyon_geom::QuadraticBezierSegment::closest_point` give exact nearest-point math
   to replace hand-rolled polyline sampling for edit-time hit tests and for
   validating the shader's cubic solve. `lyon_geom`'s `fast_bounding_box` (control
   polygon) and `bounding_box` (extrema) are exactly the conservative/tight AABBs the
   tile binning pass needs; `fat_line` offers a tighter prune if AABB lists come out
   too long. xad already hand-rolls equivalents in `crates/bezier` — adopting a crate
   is optional, but the formulas should match these references.
4. **Buffer layout from vello's playbook**: one storage buffer of per-tile
   `(offset, count)` headers plus one flat index array of edge ids. Uniform control
   flow per tile, no stack, no divergence — unlike a BVH-in-shader.
5. **Static-until-edit means CPU rebinning is free.** forma's "update only changed
   tiles" and pathfinder's CPU tiler both show the CPU-side prepass is the accepted
   cost center; at 32×32 tiles × 128 edges it is microseconds.

What to avoid:

- **A BVH in WGSL** (parry-style best-first traversal ported to the shader): parry2d
  proves the traversal pattern works and its 2-wide node layout would port, but
  stackful, divergent traversal per fragment buys nothing at n=128 (same conclusion
  as the prior doc's option (a)). Keep parry/rstar on the CPU if used at all — and at
  this n, a flat scan of 128 AABBs likely beats any tree.
- **Baked SDF textures (msdfgen-style)** for the sketch plane: resolution-bound,
  needs re-bake on every edit, and loses the exact nearest-edge *id* that face fill
  requires (msdf encodes distance, not which edge won).
- **Depending on pathfinder or forma**: both are unmaintained/archived; treat them as
  reading material.
- **Tessellation (lyon/femtovg path)** as an alternative: it eliminates the
  nearest-edge problem for *fills*, but xad's shader also uses the distance for
  edge/joint rendering and selection highlighting, so distance queries remain needed
  regardless (**uncertain** — based on the current `sketch_faces.wgsl` design intent;
  revisit if fills ever become coverage-only).
