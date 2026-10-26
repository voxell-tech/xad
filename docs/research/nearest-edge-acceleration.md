# Accelerating nearest-edge queries for sketch-face rendering

Research notes, 2026-10-13. Question: `sketch_faces.wgsl` computes, per pixel, the
signed distance to each of up to 128 quadratic Bezier edges and keeps the nearest —
O(n) per pixel. The current experiment (`examples/temp_bvh_test/voronoi.rs`) bounds
each edge's generalized-Voronoi cell with an AABB found by brute-force grid sampling
(220-wide grid). What is the principled way to get these per-edge cell bounds — or an
equivalent acceleration structure — *implicitly*, without dense sampling?

Everything below is cited; claims I could not verify against a reachable primary
source are marked **uncertain**.

---

## 1. Exact generalized Voronoi diagrams (GVD) of curved sites

The *segment Voronoi diagram* generalizes the point Voronoi diagram: the cell of a
site `t_i` is the locus of points closer to `t_i` (under `δ(x, t_i) = min_{y∈t_i} ‖x−y‖`)
than to any other site. To keep all bisectors one-dimensional and cells simply
connected, segments are decomposed into three sites: two endpoints plus the open
interior ([CGAL 2D Segment Delaunay Graph manual](https://doc.cgal.org/latest/Segment_Delaunay_graph_2/index.html)).
Bisectors between a point and a segment interior are parabolic arcs; this is why the
diagram is "generalized".

Established exact algorithms:

- **Yap (1987)** gives an O(n log n) algorithm for the Voronoi diagram of n simple
  curve segments (line segments and circular-arc pieces); the diagram has O(n)
  vertices/edges/cells. Citation: C.-K. Yap, "An O(n log n) algorithm for the
  Voronoi diagram of a set of simple curve segments", *Discrete & Computational
  Geometry* 2:365–393, 1987, <https://doi.org/10.1007/BF02187890>. **Uncertain**:
  the Springer page was behind a redirect during this research; the complexity
  claim is from the paper's standard citation, not re-verified from the PDF.
- **CGAL `Segment_Delaunay_graph_2`** (Karavelas): robust incremental construction
  of the segment Delaunay graph / Voronoi diagram for possibly-intersecting line
  segments, with geometric + arithmetic filtering for exact predicates. A point-location
  hierarchy ("Segment Delaunay graph hierarchy") supports fast nearest-site queries;
  the manual notes the hierarchy only pays off above ~1,000 sites.
  Source: <https://doc.cgal.org/latest/Segment_Delaunay_graph_2/index.html>.
  CGAL handles **line segments only**, not arcs or Beziers — Beziers would have to be
  flattened/biarc-approximated first.
- **Held's VRONI / ArcVRONI**: topology-oriented, floating-point incremental code for
  Voronoi diagrams of points, line segments, and *genuine circular arcs* (bisectors
  are genuine conic arcs); used industrially for medial axes, offsets, pocket
  machining. Reported timings: ~1,000 segments in 25 ms; 100k–450k segments in a few
  seconds (3.2 GHz Pentium 4). Source code is licensed, **not** open source.
  Source: <https://www.cosy.sbg.ac.at/~held/projects/vroni/vroni.html>.
  Key publication: Held & Huber, "Topology-Oriented Incremental Computation of Voronoi
  Diagrams of Circular Arcs and Straight Line-Segments", *CAD* 41(5):327–338, 2009,
  <https://dx.doi.org/10.1016/j.cad.2008.08.004>.

No production library computes the exact GVD of *Bezier* sites directly; the practical
route in this family is to approximate each Bezier by line segments or biarcs, compute
the segment/arc GVD, and inherit cell bounds from the pieces. The bisector curves of the
true quadratic-Bezier GVD are high-degree algebraic curves (point/Bezier and
Bezier/Bezier bisectors do not stay conic), which is why exact codes stop at arcs
(**uncertain** as a general statement — inferred from VRONI stopping at arcs with conic
bisectors; no primary source fetched stating Bezier bisector degree).

Takeaway for our use: an exact GVD gives true per-cell geometry (and hence exact AABBs
per cell), but is substantial machinery, and for n ≤ 128 the asymptotic advantage is
irrelevant.

## 2. Jump flooding (JFA) — GPU discrete Voronoi / distance transform

Rong & Tan, "Jump Flooding in GPU with Applications to Voronoi Diagram and Distance
Transform" (i3D 2006) compute an *approximate* discrete Voronoi diagram of seeds in a
2D grid in **constant number of passes per resolution**: log₂(res) full-screen passes
with step sizes res/2, res/4, …, 1, so total work O(res² · log res). Errors ("the
differences between the approximation and the actual Voronoi diagram") exist but "are
hardly noticeable to the naked eye" in their experiments; the paper quantifies error
occurrences. Source: <https://www.comp.nus.edu.sg/~tants/jfa.html> (paper PDF:
<https://www.comp.nus.edu.sg/~tants/jfa/i3d06.pdf>).

Variants (Rong & Tan, ISVD 2007, <https://www.comp.nus.edu.sg/~tants/jfa-variants.html>):
one variant **further reduces errors** (1+JFA: one extra step-1 pass), one **increases
speed**, one extends to 3D slice-by-slice; the variants are orthogonal and combinable.
(The names "1+JFA"/"JFA²" appear in the paper PDF; the project page describes them only
as "three novel variants".)

For curved sites, JFA needs the sites rasterized as seed pixels (each seed storing the
id of its source edge), so accuracy is bounded by seed rasterization density and the
texture resolution — it is a *discrete* diagram, not analytic. For our problem it
would run as a prepass into an id/texture (re-run only on edit), and the fragment
shader would do one texture fetch + one exact distance evaluation (plus optionally a
few neighbors to heal JFA's small errors). Note the per-edge AABB the current
experiment computes is exactly what one could also reduce from a JFA id-texture —
same sampling character as the CPU grid, but GPU-fast.

## 3. Vector/font rendering practice: shortlists per region, not Voronoi

This exact problem (per-pixel nearest/relevant Bezier among ~10²–10³ curves) is solved
in production renderers by **conservative binning**, not by Voronoi diagrams:

- **Nehab & Hoppe 2008, "Random-access rendering of general vector graphics"**
  (SIGGRAPH Asia): build a coarse lattice; each cell stores a variable-length
  **cell-specialized encoding of exactly the primitives it overlaps** (quadratic
  outlines supported); a pixel shader interprets the cell's list. Advantages:
  localized memory access, vector art on arbitrary surfaces, antialiasing in one
  shader invocation. Encoding is a CPU preprocess (their "hindsight" wishes for
  real-time re-encoding). Source: <https://hhoppe.com/proj/ravg/>, paper:
  <https://hhoppe.com/ravg.pdf>.
- **Slug (Lengyel)**: renders glyphs directly from quadratic Bezier outline data in a
  fragment shader, no precomputed images or distance fields; a per-font preprocess
  (`slugfont`) stores the curves in one texture and "spatial data structures that
  Slug uses for efficient rendering" in another. Sources:
  <https://sluglibrary.com/>; Lengyel, "GPU-Centered Font Rendering Directly from
  Glyph Outlines", JCGT 6(2):31–47, 2017, <https://jcgt.org/published/0006/02/02/>
  (paper + shader source freely downloadable). The precomputed structure is a set of
  horizontal/vertical **bands** per glyph, each listing (sorted) only the curves
  intersecting that band, so each fragment tests a short list — banding details are
  in the paper PDF/slides (<https://terathon.com/i3d2018_lengyel.pdf>); the project
  page itself only calls it "optimization data". Note Slug computes *coverage*
  (winding via ray-curve intersections), not nearest-curve distance, but the
  banding/shortlist idea transfers directly.
- **msdfgen (Chlumský)**: CPU preprocess that evaluates, per texel of a small
  texture, the exact signed distance to the shape's Bezier edges (median of 3
  channels preserves corners); runtime is a single texture sample + median + smooth
  step. Source: <https://github.com/Chlumsky/msdfgen>. This sidesteps the per-pixel
  nearest-curve search entirely by baking distances — i.e. it *is* dense sampling,
  done once offline at low resolution, relying on SDF interpolation for
  magnification.

Common pattern: none of these compute Voronoi cells. They compute, per spatial bin, a
**conservative candidate list** (every curve that *could* matter in that bin), which is
cheap and exact-by-construction, and let the shader do the final min over a short list.

## 4. Conservative analytic bounds without the true GVD

The standard way to bound "which edges can be nearest in region R" without building a
GVD is branch-and-bound pruning, as used in BVH/KD-tree nearest-neighbor search and in
msdfgen-style distance evaluation:

- For region R (a tile or BVH node) and edge set E, compute an **upper bound**
  `U(R) = min over edges e of max-distance(R, e)` — e.g. the max distance from R to
  some anchor on the easiest edge, or `dist(center(R), e) + radius(R)`.
- Any edge whose **lower bound** `dist(R, AABB(e)) > U(R)` cannot be nearest anywhere
  in R and is pruned.

This is elementary metric-space pruning (triangle inequality), not attributable to one
paper; it is precisely what a BVH nearest-primitive traversal does per query point, and
what tile-binning does per region. Quadratic Beziers make the bounds easy: the convex
hull / control-polygon AABB of a quadratic Bezier contains the curve, giving a valid
lower bound; the distance to either endpoint gives a valid upper bound. (Exact
point-to-quadratic-Bezier distance itself requires solving a cubic — this is what the
shader already does per edge.)

Important subtlety vs. the current experiment: a per-edge **Voronoi-cell AABB** is the
*inverse* mapping (edge → region where it wins). The conservative-list approach inverts
it (region → edges that can win), which is strictly easier to compute exactly: cell
AABBs from sampling can *miss* slivers of a cell (incorrect rendering), while a
conservative per-tile list can only be *too long* (slower, never wrong). Interval
arithmetic can certify the same pruning bounds over a whole tile, but for AABB-vs-AABB
distance bounds plain min/max arithmetic already is the interval computation.

## 5. Comparison for our constraints

Constraints: ≤128 quadratic Beziers, static between edits, query = every fragment of a
quad per frame, WGSL/bevy, correctness of the nearest-edge id matters (it drives face
fill), and the CPU already touches the data on every edit.

| Approach | Preprocess (per edit) | Per-fragment cost | Correctness | Fit |
|---|---|---|---|---|
| (a) BVH over edge AABBs, branch-and-bound in shader | O(n log n) CPU build, tiny for n=128 | log-ish traversal with stack + pruned cubic solves; divergent branching | Exact | Good; but WGSL stack/divergence overhead can eat the win at n=128 |
| (b) Tile/grid binning with conservative candidate lists (Nehab–Hoppe / Slug-style bands) | O(tiles · n) trivial CPU pass | 1 lookup + k cubic solves, k = list length; uniform control flow | Exact (lists are conservative) | **Best fit** — exactly what RAVG/Slug do in production ([RAVG](https://hhoppe.com/proj/ravg/), [Slug](https://jcgt.org/published/0006/02/02/)) |
| (c) JFA prepass to id-texture | log₂(res) full-screen GPU passes on edit ([Rong & Tan](https://www.comp.nus.edu.sg/~tants/jfa.html)) | 1 texture fetch (+1 exact distance) | Approximate (small, quantified errors; 1+JFA reduces them) | Workable; resolution-dependent, needs seed rasterization of curves, errors possible at cell boundaries |
| (d) Exact GVD (CGAL SDG / VRONI) | ms-scale even for 10³ segments ([VRONI](https://www.cosy.sbg.ac.at/~held/projects/vroni/vroni.html)); Beziers must be flattened; CGAL is segments-only, VRONI is closed-source | 1 cell lookup (point location) or baked cell AABBs | Exact for the *approximated* curves only | Overkill at n=128; approximation + dependency weight |
| (current) CPU grid sampling of cell AABBs | O(grid · n) | AABB test + shortlist | **Not conservative** — can misclassify near unsampled slivers | Works, but resolution-bound and can be wrong |

## 6. Recommendation

For ≤128 static quadratic Beziers rendered by a fragment shader, the literature's answer
is **(b): drop the per-edge Voronoi-cell AABBs and invert the problem — bin the plane
into a coarse uniform grid of tiles and store, per tile, the conservative list of edges
that can be nearest somewhere in that tile.** This is the Nehab–Hoppe cell-specialized
shortlist ([RAVG](https://hhoppe.com/proj/ravg/)) / Slug banding
([JCGT 6(2)](https://jcgt.org/published/0006/02/02/)) pattern, adapted from
"curves overlapping the cell" to "curves possibly nearest in the cell":

1. Per edge, precompute its control-polygon AABB (contains the curve — convex hull
   property) and endpoints.
2. Per tile R: `U(R) = min_e maxDist(R, e)` using a cheap upper bound (e.g. farthest
   corner of R to an endpoint of e); keep every edge with
   `dist(R, AABB(e)) ≤ U(R)`. Section 4's triangle-inequality argument makes the
   list conservative, so rendering stays exact regardless of tile size.
3. Pack lists into a storage buffer (offsets + edge indices, like RAVG's
   variable-length cell encodings); fragment shader maps `frag → tile`, loops the
   short list with the existing cubic distance solve. Control flow is uniform per
   tile — no stack, no divergence, WGSL-friendly.
4. Rebuild on edit only: a 32×32 grid × 128 edges is ~131k AABB-distance checks,
   microseconds on CPU; no GPU prepass needed.

Why not the others here:

- (a) BVH-in-shader is exact too, but stack traversal + divergence in WGSL buys little
  at n=128; it becomes attractive if n grows to thousands.
- (c) JFA replaces one sampling scheme with another (better: GPU-fast, log passes;
  worse: still resolution-bound, small boundary errors, and curve seeds must be
  rasterized densely — Rong & Tan seed from pixels, not analytic curves).
- (d) Exact GVD machinery (VRONI/CGAL) is designed for 10⁴–10⁵ sites with robustness
  guarantees; at n=128 its construction answers a harder question than needed, can't
  take Beziers natively, and VRONI isn't freely licensable.

If per-edge cell AABBs are still wanted (e.g. for visualization), derive them from the
tile lists: an edge's cell is contained in the union of tiles whose list contains it —
a conservative (never-undershooting) bound, unlike the sampled one.
