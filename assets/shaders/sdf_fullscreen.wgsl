// SDF Raymarching Shader using FullscreenMaterial
// Note: bindings 0 and 1 are reserved for screen_texture/sampler (we ignore them)

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct SdfUniforms {
    camera_pos: vec4f,
    camera_dir: vec4f,
    camera_up: vec4f,
    resolution: vec2f,
    fov: f32,
    _padding: f32,
};

// Binding 2 because 0 and 1 are reserved for screen_texture in FullscreenMaterial
@group(0) @binding(2)
var<uniform> uniforms: SdfUniforms;

// SDF primitives - https://iquilezles.org/articles/distfunctions/
fn sdSphere(p: vec3f, r: f32) -> f32 {
    return length(p) - r;
}

fn sdBox(p: vec3f, b: vec3f) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sdRoundBox(p: vec3f, b: vec3f, r: f32) -> f32 {
    let q = abs(p) - b + r;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

fn sdTorus(p: vec3f, t: vec2f) -> f32 {
    let q = vec2f(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

// Scene composition
fn map(p: vec3f) -> f32 {
    let box_dist = sdRoundBox(p, vec3f(1.0), 0.1);
    let sphere_dist = sdSphere(p - vec3f(2.5, 0.0, 0.0), 0.8);
    let torus_dist = sdTorus(p - vec3f(-2.5, 0.0, 0.0), vec2f(0.8, 0.3));

    return min(min(box_dist, sphere_dist), torus_dist);
}

// Calculate surface normal via gradient
fn calcNormal(p: vec3f) -> vec3f {
    let e = vec2f(0.0001, 0.0);
    return normalize(vec3f(
        map(p + e.xyy) - map(p - e.xyy),
        map(p + e.yxy) - map(p - e.yxy),
        map(p + e.yyx) - map(p - e.yyx)
    ));
}

// Soft shadows
fn calcSoftShadow(ro: vec3f, rd: vec3f, mint: f32, maxt: f32, k: f32) -> f32 {
    var res = 1.0;
    var t = mint;
    for (var i = 0; i < 64; i++) {
        let h = map(ro + rd * t);
        res = min(res, k * h / t);
        t += clamp(h, 0.02, 0.10);
        if res < 0.001 || t > maxt {
            break;
        }
    }
    return clamp(res, 0.0, 1.0);
}

// Ambient occlusion
fn calcAO(pos: vec3f, nor: vec3f) -> f32 {
    var occ = 0.0;
    var sca = 1.0;
    for (var i = 0; i < 5; i++) {
        let h = 0.01 + 0.12 * f32(i) / 4.0;
        let d = map(pos + h * nor);
        occ += (h - d) * sca;
        sca *= 0.95;
    }
    return clamp(1.0 - 3.0 * occ, 0.0, 1.0);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    // Convert UV [0,1] to NDC [-1,1]
    let uv = in.uv * 2.0 - 1.0;

    // Calculate aspect ratio
    let aspect = uniforms.resolution.x / uniforms.resolution.y;

    // Build camera ray
    let camera_right = normalize(cross(uniforms.camera_dir.xyz, uniforms.camera_up.xyz));
    let camera_up = normalize(cross(camera_right, uniforms.camera_dir.xyz));

    let fov_scale = tan(uniforms.fov);
    let ray_dir = normalize(
        uniforms.camera_dir.xyz +
        uv.x * aspect * fov_scale * camera_right +
        uv.y * fov_scale * camera_up
    );

    let ray_origin = uniforms.camera_pos.xyz;

    // Raymarching
    var t = 0.0;
    var hit = false;

    for (var i = 0; i < 128; i++) {
        let p = ray_origin + ray_dir * t;
        let d = map(p);

        if d < 0.0001 {
            hit = true;
            break;
        }

        t += d;

        if t > 100.0 {
            break;
        }
    }

    if hit {
        let pos = ray_origin + ray_dir * t;
        let normal = calcNormal(pos);

        // Simple lighting
        let light_dir = normalize(vec3f(1.0, 1.0, 1.0));
        let diffuse = max(dot(normal, light_dir), 0.0);
        let ambient = 0.1;

        // Soft shadow
        let shadow = calcSoftShadow(pos + normal * 0.001, light_dir, 0.01, 10.0, 8.0);

        // Ambient occlusion
        let ao = calcAO(pos, normal);

        // Base color from normal
        let base_color = normal * 0.5 + 0.5;

        // Final color
        let color = base_color * (ambient + diffuse * shadow) * ao;

        return vec4f(color, 1.0);
    }

    // Background gradient
    let bg = mix(vec3f(0.1, 0.1, 0.15), vec3f(0.3, 0.3, 0.4), in.uv.y);
    return vec4f(bg, 1.0);
}
