#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View;

struct SdfCamera {
    max_step: u32,
    far_plane: f32,
};

struct SdfTransformUniform {
    local_from_world: mat3x4<f32>,
    scale: f32,
};


@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> sdf_camera: SdfCamera;
@group(0) @binding(4) var<storage> sdf_transforms: array<SdfTransformUniform>;

// SDF primitives - https://iquilezles.org/articles/distfunctions/
fn sd_sphere(p: vec3f, r: f32) -> f32 {
    return length(p) - r;
}

fn sd_box(p: vec3f, b: vec3f) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sd_round_box(p: vec3f, b: vec3f, r: f32) -> f32 {
    let q = abs(p) - b + r;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

fn sd_torus(p: vec3f, t: vec2f) -> f32 {
    let q = vec2f(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

/// Scene composition.
fn composition(point: vec3f) -> f32 {
    let len = arrayLength(&sdf_transforms);
    var dist = sdf_camera.far_plane;

    // TODO: Implement acceleration structure (BVH?) to prevent looping over
    // the entire transform buffer.
    // TODO: Support different SDF primitives.
    for (var i = 0u; i < len; i++) {
        let transform = sdf_transforms[i];
        let sample_point = (vec4f(point, 1.0) * transform.local_from_world).xyz;
        dist = min(dist, sd_box(sample_point / transform.scale, vec3f(0.3)) * transform.scale);
    }

    return dist;
}

/// Calculate surface normal via gradient.
fn calc_normal(p: vec3f) -> vec3f {
    let e = vec2f(0.0001, 0.0);
    return normalize(vec3f(
        composition(p + e.xyy) - composition(p - e.xyy),
        composition(p + e.yxy) - composition(p - e.yxy),
        composition(p + e.yyx) - composition(p - e.yyx)
    ));
}

/// Soft shadows.
fn calc_soft_shadow(ro: vec3f, rd: vec3f, mint: f32, maxt: f32, k: f32) -> f32 {
    var res = 1.0;
    var t = mint;
    for (var i = 0; i < 64; i++) {
        let h = composition(ro + rd * t);
        res = min(res, k * h / t);
        t += clamp(h, 0.02, 0.1);
        if res < 0.001 || t > maxt {
            break;
        }
    }
    return clamp(res, 0.0, 1.0);
}

/// Ambient occlusion.
fn calc_ao(pos: vec3f, nor: vec3f) -> f32 {
    var occ = 0.0;
    var sca = 1.0;
    for (var i = 0; i < 5; i++) {
        let h = 0.01 + 0.12 * f32(i) / 4.0;
        let d = composition(pos + h * nor);
        occ += (h - d) * sca;
        sca *= 0.85;
    }
    return clamp(1.0 - 3.0 * occ, 0.0, 1.0);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    // Convert UV [0,1] to NDC [-1,1].
    let ndc = vec2f(in.uv.x * 2.0 - 1.0, in.uv.y * -2.0 + 1.0);

    let ray_origin = view.world_position;

    // Calculate ray direction using the `world_from_clip` matrix.
    // We pick a point on the far plane (z = 0.0 in reverse-z, or 1.0 in standard)
    // For hit-testing, any Z works as long as we normalize the resulting vector.
    let target_clip = vec4f(ndc, 1.0, 1.0); 
    let target_world_homogenous = view.world_from_clip * target_clip;
    let target_world = target_world_homogenous.xyz / target_world_homogenous.w;

    let ray_dir = normalize(target_world - ray_origin);

    // Raymarching.
    var march = 0.0;
    var hit = false;

    for (var i = 0u; i < sdf_camera.max_step; i++) {
        let p = ray_origin + ray_dir * march;
        let d = composition(p);

        if d < 0.001 {
            hit = true;
            break;
        }

        march += d;

        if march >= sdf_camera.far_plane {
            break;
        }
    }

    if hit {
        let pos = ray_origin + ray_dir * march;
        let normal = calc_normal(pos);

        // Simple lighting.
        let light_dir = normalize(vec3f(1.0, 1.0, 1.0));
        let diffuse = max(dot(normal, light_dir), 0.0);
        let ambient = 0.1;

        // Soft shadow.
        let shadow = calc_soft_shadow(pos + normal * 0.001, light_dir, 0.01, 10.0, 8.0);

        // Ambient occlusion.
        let ao = calc_ao(pos, normal);

        // Base color from normal.
        let base_color = normal * 0.7 + 0.3;

        // Final color.
        let color = base_color * (ambient + diffuse * shadow) * ao;

        return vec4f(color, 1.0);
    }

    // Background.
    // TODO: Utilize depth texture to merge sdf!
    return textureSample(screen_texture, texture_sampler, in.uv);
}
