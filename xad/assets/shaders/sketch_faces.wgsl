#import bevy_pbr::forward_io::VertexOutput

// A curve with the face touching each side, looking along p0 -> p2.
// 0 = the unbounded outside, otherwise face id + 1.
struct Edge {
    p0:    vec2f,
    p1:    vec2f,
    p2:    vec2f,
    left:  u32,
    right: u32,
}

struct FaceBuffer {
    // Plane origin and basis: maps world positions into sketch coordinates.
    origin:     vec4f,
    tangent:    vec4f,
    bitangent:  vec4f,
    edge_count: u32,
    colors:     array<vec4f, 32>,
    edges:      array<Edge, 128>,
}

@group(3) @binding(0) var<storage, read> data: FaceBuffer;

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

// Signed distance to a quadratic Bezier: negative left of p0 -> p2 travel,
// positive right.
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

// Distinct tint per edge index, lifted toward white to stay readable.
fn edge_tint(i: u32) -> vec3f {
    // PCG hash
    var s = i * 747796405u + 2891336453u;
    s = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    s = (s >> 22u) ^ s;
    let rgb = vec3f(
        f32(s & 255u),
        f32((s >> 8u) & 255u),
        f32((s >> 16u) & 255u),
    ) / 255.0;
    return 0.35 + 0.65 * rgb;
}

// Tint by nearest edge instead of face color, exposing where each edge's
// nearest-region begins and ends. Ties between near-equidistant edges dither.
const DEBUG_NEAREST_EDGE: bool = false;

// Endpoints this close count as the same vertex (2x the CPU weld distance).
const VERTEX_EPS: f32 = 4e-3;
const NO_SECTOR: u32 = 0xffffffffu;

// Angle swept rotating counter-clockwise from u to d, in [0, 2pi).
fn ccw_angle(u: vec2f, d: vec2f) -> f32 {
    let a = atan2(cross2(u, d), dot(u, d));
    return select(a, a + 6.2831853, a < 0.0);
}

// Face of the angular sector around vertex `v` that contains `p`.
//
// Rays leave v along every edge-end that touches it. The sector holding
// d = p - v is bounded clockwise by the ray with the smallest CCW angle to
// d, and the sector's face lies on that ray's CCW side: `left` for an edge
// leaving v forward, `right` for one traversed backward.
fn sector_face(p: vec2f, v: vec2f) -> u32 {
    let d = p - v;
    var best = 7.0; // > 2pi
    var face = NO_SECTOR;

    for (var i = 0u; i < data.edge_count; i++) {
        let e = data.edges[i];

        if (distance(e.p0, v) < VERTEX_EPS) {
            var u = e.p1 - e.p0;
            if (dot(u, u) < 1e-12) {
                u = e.p2 - e.p0;
            }
            let angle = ccw_angle(u, d);
            if (angle < best) {
                best = angle;
                face = e.left;
            }
        }
        if (distance(e.p2, v) < VERTEX_EPS) {
            var u = e.p1 - e.p2;
            if (dot(u, u) < 1e-12) {
                u = e.p0 - e.p2;
            }
            let angle = ccw_angle(u, d);
            if (angle < best) {
                best = angle;
                face = e.right;
            }
        }
    }

    return face;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4f {
    let rel = in.world_position.xyz - data.origin.xyz;
    let p = vec2f(dot(rel, data.tangent.xyz), dot(rel, data.bitangent.xyz));

    if (data.edge_count == 0u) {
        return vec4f(0.0);
    }

    // Find the nearest edge.
    // TODO: O(n) per pixel; a spatial structure (grid / BVH) would cut this.
    var nearest = 1e20;
    var index = 0u;
    for (var i = 0u; i < data.edge_count; i++) {
        let edge = data.edges[i];
        let sd = sdf_bezier_quadratic(p, edge.p0, edge.p1, edge.p2);
        if (abs(sd) < abs(nearest)) {
            nearest = sd;
            index = i;
        }
    }

    if (DEBUG_NEAREST_EDGE) {
        return vec4f(edge_tint(index), 0.85);
    }

    // The sign tells which side of the edge this pixel is on — except when
    // the nearest point is the edge's endpoint: past a sharp vertex, both
    // of its edges' tangent half-planes can claim the face side. There the
    // vertex's angular sectors decide instead.
    let edge = data.edges[index];
    var face = select(edge.right, edge.left, nearest < 0.0);
    let near_vertex =
        min(distance(p, edge.p0), distance(p, edge.p2));
    if (near_vertex < abs(nearest) + 1e-4) {
        let v = select(
            edge.p2,
            edge.p0,
            distance(p, edge.p0) < distance(p, edge.p2),
        );
        let sector = sector_face(p, v);
        if (sector != NO_SECTOR) {
            face = sector;
        }
    }

    if (face == 0u) {
        // The unbounded outside: draw nothing.
        return vec4f(0.0);
    }
    return data.colors[face - 1u];
}
