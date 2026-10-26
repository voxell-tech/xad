// Raymarched CSG over extruded sketch faces.
//
// Each leaf is one face swept along its plane normal, using iq's
// opExtrusion (https://www.shadertoy.com/view/4lyfzw) over the exact
// signed distance to the face boundary. The postfix node list combines
// leaves with min/max booleans.
//
// Every op here returns a LOWER bound on the true distance (union's min
// is exact outside; a subtraction or intersection is contained in each
// operand, so the larger of their distances still underestimates). The
// march can therefore only understep, never tunnel.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct Node {
    kind:  u32,
    index: u32,
}

struct Curve {
    p0: vec2f,
    p1: vec2f,
    p2: vec2f,
}

struct Solid {
    origin:        vec4f,
    tangent:       vec4f,
    bitangent:     vec4f,
    normal:        vec4f,
    grid_min:      vec2f,
    inv_tile_size: vec2f,
    extent_min:    f32,
    extent_max:    f32,
    grid_cols:     u32,
    grid_rows:     u32,
    curve_base:    u32,
    tile_base:     u32,
    index_base:    u32,
    _pad:          u32,
}

struct Params {
    color:      vec4f,
    bounds_min: vec4f,
    bounds_max: vec4f,
    node_count: u32,
}

@group(3) @binding(0) var<uniform> params: Params;
@group(3) @binding(1) var<storage, read> nodes: array<Node>;
@group(3) @binding(2) var<storage, read> solids: array<Solid>;
@group(3) @binding(3) var<storage, read> curves: array<Curve>;
@group(3) @binding(4) var<storage, read> tiles: array<vec2u>;
@group(3) @binding(5) var<storage, read> tile_indices: array<u32>;

const NODE_SOLID: u32 = 0u;
const OP_UNION: u32 = 0u;
const OP_SUBTRACT: u32 = 1u;
const OP_INTERSECT: u32 = 2u;

// Deepest postfix operand stack supported.
// TODO: report the volume's stack depth and reject deeper trees on the
// CPU instead of silently clamping here.
const MAX_STACK: u32 = 16u;

const MAX_STEPS: i32 = 256;
// Hit tolerance, grown with distance to cut grazing-angle banding.
const HIT_EPS: f32 = 2e-4;
// The body's base face is coplanar with its sketch plane, so its depth
// ties with the sketch quads. Reverse-Z: larger is nearer.
const DEPTH_BIAS: f32 = 1e-5;

// https://www.shadertoy.com/view/MlKcDD
const SQRT3: f32 = 1.732050807568877;

fn cross2(a: vec2f, b: vec2f) -> f32 {
    return a.x * b.y - a.y * b.x;
}

// Signed distance to a segment: negative left of a -> b, positive right.
fn sdf_line_partition(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let ba = b - a;
    let pa = p - a;
    let h = saturate(dot(pa, ba) / dot(ba, ba));
    let k = pa - h * ba;
    let n = vec2f(ba.y, -ba.x);
    if (dot(k, n) >= 0.0) {
        return length(k);
    }
    return -length(k);
}

// Signed distance to a quadratic Bezier: negative left of p0 -> p2
// travel, positive right.
fn sdf_bezier_quadratic(pos: vec2f, A: vec2f, B: vec2f, C: vec2f) -> f32 {
    let EPSILON = 1e-3;
    let ONE_THIRD = 1.0 / 3.0;

    let ab_equal = all(A == B);
    let bc_equal = all(B == C);
    let ac_equal = all(A == C);

    if (ab_equal && bc_equal) {
        return distance(pos, A);
    } else if (ab_equal || ac_equal) {
        return sdf_line_partition(pos, B, C);
    } else if (bc_equal) {
        return sdf_line_partition(pos, A, C);
    }

    if (abs(dot(normalize(B - A), normalize(C - B)) - 1.0) < EPSILON) {
        return sdf_line_partition(pos, A, C);
    }

    let a = B - A;
    let b = A - 2.0 * B + C;
    let c = a * 2.0;
    let d = A - pos;

    let kk = 1.0 / dot(b, b);
    let kx = kk * dot(a, b);
    let ky = kk * (2.0 * dot(a, a) + dot(d, b)) * ONE_THIRD;
    let kz = kk * dot(d, a);

    var res = 0.0;
    var sgn = 0.0;

    let p = ky - kx * kx;
    let p3 = p * p * p;
    let q = kx * (2.0 * kx * kx - 3.0 * ky) + kz;
    let h = q * q + 4.0 * p3;

    if (h >= 0.0) {
        // One real root
        let hs = sqrt(h);
        let x = 0.5 * (vec2f(hs, -hs) - q);
        let uv = sign(x) * pow(abs(x), vec2f(ONE_THIRD));
        let t = saturate(uv.x + uv.y - kx) + EPSILON;
        let qv = d + (c + b * t) * t;
        res = dot(qv, qv);
        sgn = cross2(c + 2.0 * b * t, qv);
    } else {
        // Three real roots
        let z = sqrt(-p);
        let v = acos(q / (p * z * 2.0)) * ONE_THIRD;
        let m = cos(v);
        let n = sin(v) * SQRT3;
        let t = saturate(vec3f(m + m, -n - m, n - m) * z - kx) + EPSILON;
        let qx = d + (c + b * t.x) * t.x;
        let dx = dot(qx, qx);
        let sx = cross2(c + 2.0 * b * t.x, qx);
        let qy = d + (c + b * t.y) * t.y;
        let dy = dot(qy, qy);
        let sy = cross2(c + 2.0 * b * t.y, qy);
        if (dx < dy) {
            res = dx;
            sgn = sx;
        } else {
            res = dy;
            sgn = sy;
        }
    }

    return sign(sgn) * sqrt(res);
}

// Endpoints this close count as the same vertex (2x the CPU weld
// distance).
const VERTEX_EPS: f32 = 4e-3;

// Angle swept rotating counter-clockwise from u to d, in [0, 2pi).
fn ccw_angle(u: vec2f, d: vec2f) -> f32 {
    let a = atan2(cross2(u, d), dot(u, d));
    return select(a, a + 6.2831853, a < 0.0);
}

// Whether `p` lies inside the face, decided by the angular sector of
// vertex `v` that holds it: 1 inside, 0 outside, -1 undecided.
//
// Rays leave v along every curve-end touching it. The sector holding
// d = p - v is bounded clockwise by the ray at the smallest CCW angle,
// and every curve keeps the face on its left, so a ray leaving v opens
// an inside sector and one arriving at v opens an outside sector.
//
// Only the tile's candidates are scanned: a curve touching v is within
// distance(p, v) of p, so the tile list must already hold it.
fn sector_inside(p: vec2f, v: vec2f, s: Solid, tile: vec2u) -> i32 {
    let d = p - v;
    var best = 7.0; // > 2pi
    var inside = -1;

    for (var k = 0u; k < tile.y; k++) {
        let index = tile_indices[s.index_base + tile.x + k];
        let c = curves[s.curve_base + index];

        if (distance(c.p0, v) < VERTEX_EPS) {
            var u = c.p1 - c.p0;
            if (dot(u, u) < 1e-12) {
                u = c.p2 - c.p0;
            }
            let angle = ccw_angle(u, d);
            if (angle < best) {
                best = angle;
                inside = 1;
            }
        }
        if (distance(c.p2, v) < VERTEX_EPS) {
            var u = c.p1 - c.p2;
            if (dot(u, u) < 1e-12) {
                u = c.p0 - c.p2;
            }
            let angle = ccw_angle(u, d);
            if (angle < best) {
                best = angle;
                inside = 0;
            }
        }
    }

    return inside;
}

// Signed distance to a solid's face boundary, negative inside the face.
//
// Inside the grid the point's tile lists every curve that can be nearest
// there, so this is exact. Outside, bound it from the clamped point q:
// the true distance is at least |p - q| (every curve lies inside the
// grid) and at least sd(q) - |p - q| (triangle inequality).
fn sd_face(p: vec2f, s: Solid) -> f32 {
    let grid_size =
        vec2f(f32(s.grid_cols), f32(s.grid_rows)) / s.inv_tile_size;
    let q = clamp(p, s.grid_min, s.grid_min + grid_size);

    let cell = clamp(
        vec2i((q - s.grid_min) * s.inv_tile_size),
        vec2i(0),
        vec2i(i32(s.grid_cols) - 1, i32(s.grid_rows) - 1),
    );
    let tile = tiles[
        s.tile_base + u32(cell.y) * s.grid_cols + u32(cell.x)
    ];

    var best = 1e20;
    var nearest = Curve(vec2f(0.0), vec2f(0.0), vec2f(0.0));
    for (var k = 0u; k < tile.y; k++) {
        let index = tile_indices[s.index_base + tile.x + k];
        let c = curves[s.curve_base + index];
        let sd = sdf_bezier_quadratic(q, c.p0, c.p1, c.p2);
        if (abs(sd) < abs(best)) {
            best = sd;
            nearest = c;
        }
    }

    // The nearest curve's sign is only trustworthy away from its ends:
    // past a sharp vertex both adjoining curves claim the same side,
    // which on a deeply concave profile spills far outside the face.
    // There the vertex's angular sectors decide instead.
    let to_p0 = distance(q, nearest.p0);
    let to_p2 = distance(q, nearest.p2);
    if (min(to_p0, to_p2) < abs(best) + 1e-4) {
        let v = select(nearest.p2, nearest.p0, to_p0 < to_p2);
        let inside = sector_inside(q, v, s, tile);
        if (inside >= 0) {
            best = select(abs(best), -abs(best), inside == 1);
        }
    }

    let gap = length(p - q);
    if (gap > 0.0) {
        return max(gap, best - gap);
    }
    return best;
}

// iq's opExtrusion, with the sweep as a slab along the plane normal.
fn sd_solid(p: vec3f, index: u32) -> f32 {
    let s = solids[index];
    let rel = p - s.origin.xyz;
    let local = vec3f(
        dot(rel, s.tangent.xyz),
        dot(rel, s.bitangent.xyz),
        dot(rel, s.normal.xyz),
    );

    let face = sd_face(local.xy, s);
    let slab = max(s.extent_min - local.z, local.z - s.extent_max);
    let w = vec2f(face, slab);
    return min(max(w.x, w.y), 0.0) + length(max(w, vec2f(0.0)));
}

fn apply_op(op: u32, body: f32, tool: f32) -> f32 {
    switch op {
        case OP_SUBTRACT:  { return max(body, -tool); }
        case OP_INTERSECT: { return max(body, tool); }
        default:           { return min(body, tool); }
    }
}

// Evaluates the postfix node list: a leaf pushes, an op folds the top two.
fn sd_volume(p: vec3f) -> f32 {
    var stack: array<f32, 16>;
    var sp = 0u;

    for (var i = 0u; i < params.node_count; i++) {
        let node = nodes[i];
        if (node.kind == NODE_SOLID) {
            if (sp < MAX_STACK) {
                stack[sp] = sd_solid(p, node.index);
                sp += 1u;
            }
        } else if (sp >= 2u) {
            let tool = stack[sp - 1u];
            sp -= 1u;
            stack[sp - 1u] =
                apply_op(node.index, stack[sp - 1u], tool);
        }
    }

    if (sp == 0u) {
        return 1e20;
    }
    return stack[0];
}

// Tetrahedron gradient.
fn normal_at(p: vec3f) -> vec3f {
    let e = vec2f(1.0, -1.0) * 5e-4;
    return normalize(
        e.xyy * sd_volume(p + e.xyy)
            + e.yyx * sd_volume(p + e.yyx)
            + e.yxy * sd_volume(p + e.yxy)
            + e.xxx * sd_volume(p + e.xxx),
    );
}

struct FragmentOutput {
    // The hit's depth, not the proxy box's, so the box never occludes
    // what lies between it and the surface.
    @builtin(frag_depth) depth: f32,
    @location(0) color: vec4f,
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let ro = view.world_position;
    let rd = normalize(in.world_position.xyz - ro);

    // Slab test for the ray's span inside the proxy box. Taken
    // analytically rather than from the entry face, so the march is
    // right even when the camera sits inside the box.
    let inv = 1.0 / rd;
    let a = (params.bounds_min.xyz - ro) * inv;
    let b = (params.bounds_max.xyz - ro) * inv;
    let near = min(a, b);
    let far = max(a, b);
    let t_near = max(max(near.x, near.y), near.z);
    let t_far = min(min(far.x, far.y), far.z);
    if (t_near > t_far || t_far < 0.0) {
        discard;
    }

    var t = max(t_near, 0.0);
    var hit = false;
    for (var i = 0; i < MAX_STEPS; i++) {
        let p = ro + rd * t;
        let d = sd_volume(p);
        if (d < HIT_EPS * max(t, 1.0)) {
            hit = true;
            break;
        }
        t += d;
        if (t > t_far) {
            break;
        }
    }
    if (!hit) {
        discard;
    }

    let p = ro + rd * t;
    let n = normal_at(p);
    let key = normalize(vec3f(0.5, 0.8, 0.6));
    let fill = normalize(vec3f(-0.6, 0.2, -0.4));
    let light = 0.18
        + 0.75 * max(dot(n, key), 0.0)
        + 0.25 * max(dot(n, fill), 0.0);
    let clip = view.clip_from_world * vec4f(p, 1.0);

    var out: FragmentOutput;
    out.depth = clip.z / clip.w + DEPTH_BIAS;
    out.color = vec4f(params.color.rgb * light, 1.0);
    return out;
}
