#import bevy_pbr::forward_io::VertexOutput

struct BezierCurve {
    p0:    vec2f,
    p1:    vec2f,
    p2:    vec2f,
    color: vec4f,
    width: f32,
    // non-zero = draw control points
    debug: u32,
    // non-zero = draw a dot at each endpoint (p0 and p2)
    draw_endpoints: u32,
    endpoint_radius: f32,
}

struct BezierBuffer {
    background_color:   vec4f,
    curve_count:        u32,
    // TODO: recalculate padding for byte offset
    _padding:           array<f32, 3>,
    curves:             array<BezierCurve, 32>,
}

@group(3) @binding(0) var<storage, read> data: BezierBuffer;


// https://www.shadertoy.com/view/MlKcDD
const SQRT3: f32 = 1.732050807568877;

fn cross2(a: vec2f, b: vec2f) -> f32 {
    return a.x * b.y - a.y * b.x;
}

// Signed distance to a line segment
// Either positive or negative, depending on which side the point falls
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

// DEBUG
// hard-coded values, doesnt matter
const DEBUG_POINT_RADIUS: f32 = 0.006;
const DEBUG_LINE_WIDTH: f32 = 0.0012;
const DEBUG_POINT_COLOR: vec4f = vec4f(1.0, 0.0, 0.0, 0.5);
const DEBUG_LINE_COLOR: vec4f = vec4f(1.0, 1.0, 1.0, 0.35);

fn draw_debug_point(color: vec3f, uv: vec2f, p: vec2f) -> vec3f {
    let d    = distance(uv, p);
    let aa   = fwidth(d);
    let mask = 1.0 - smoothstep(DEBUG_POINT_RADIUS - aa, DEBUG_POINT_RADIUS + aa, d);
    return mix(color, DEBUG_POINT_COLOR.rgb, mask * DEBUG_POINT_COLOR.a);
}

fn draw_debug_line(color: vec3f, uv: vec2f, a: vec2f, b: vec2f) -> vec3f {
    let d    = abs(sdf_line_partition(uv, a, b));
    let aa   = fwidth(d);
    let mask = 1.0 - smoothstep(DEBUG_LINE_WIDTH - aa, DEBUG_LINE_WIDTH + aa, d);
    return mix(color, DEBUG_LINE_COLOR.rgb, mask * DEBUG_LINE_COLOR.a);
}

// Draws the control points, and the lines between them
fn draw_debug_overlay(color: vec3f, uv: vec2f, curve: BezierCurve) -> vec3f {
    var c = color;
    c = draw_debug_line(c, uv, curve.p0, curve.p1);
    c = draw_debug_line(c, uv, curve.p1, curve.p2);
    c = draw_debug_point(c, uv, curve.p0);
    c = draw_debug_point(c, uv, curve.p1);
    c = draw_debug_point(c, uv, curve.p2);
    return c;
}
// 

fn draw_endpoint(canvas: vec4f, uv: vec2f, p: vec2f, radius: f32, dot_color: vec4f) -> vec4f {
    let d        = distance(uv, p);
    let aa       = fwidth(d);
    let mask     = 1.0 - smoothstep(radius - aa, radius + aa, d);
    let coverage = mask * dot_color.a;
    let color    = mix(canvas.rgb, dot_color.rgb, coverage);
    let alpha    = mix(canvas.a, 1.0, coverage);
    return vec4f(color, alpha);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4f {
    let uv   = in.uv;
    var color = data.background_color.rgb;
    var alpha = data.background_color.a;

    for (var i = 0u; i < data.curve_count; i++) {
        let curve = data.curves[i];
        let d = abs(sdf_bezier_quadratic(uv, curve.p0, curve.p1, curve.p2));
        let aa   = fwidth(d);
        let mask = 1.0 - smoothstep(curve.width - aa, curve.width + aa, d);
        let coverage = mask * curve.color.a;
        color    = mix(color, curve.color.rgb, coverage);
        alpha    = mix(alpha, 1.0, coverage);

        if (curve.debug != 0u) {
            color = draw_debug_overlay(color, uv, curve);
        }

        if (curve.draw_endpoints != 0u) {
            var canvas = vec4f(color, alpha);
            canvas = draw_endpoint(canvas, uv, curve.p0, curve.endpoint_radius, curve.color);
            canvas = draw_endpoint(canvas, uv, curve.p2, curve.endpoint_radius, curve.color);
            color = canvas.rgb;
            alpha = canvas.a;
        }
    }

    return vec4f(color, alpha);
}

