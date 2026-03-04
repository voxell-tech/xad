#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View;

struct SdfCamera {
    max_step: u32,
    far_plane: f32,
};

struct SdfInput {
    local_from_world: mat3x4<f32>,
    scale: f32,
    primitive_type: u32,
    primitive_index: u32,
};

struct SdfSphere {
    radius: f32,
}

struct SdfCuboid {
    extents: vec3f,
}

struct SdfRoundCuboid {
    extents: vec3f,
    radius: f32,
}

struct SdfCapsule {
    point_a: vec3f,
    point_b: vec3f,
    radius: f32,
}

const SPHERE: u32 = 0;
const CUBOID: u32 = 1;
const ROUND_CUBOID: u32 = 2;
const CAPSULE: u32 = 3;

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> sdf_camera: SdfCamera;
@group(0) @binding(4) var<storage> inputs: array<SdfInput>;
@group(0) @binding(5) var<storage> spheres: array<SdfSphere>;
@group(0) @binding(6) var<storage> cuboids: array<SdfCuboid>;
@group(0) @binding(7) var<storage> round_cuboids: array<SdfRoundCuboid>;
@group(0) @binding(8) var<storage> capsules: array<SdfCapsule>;

// SDF primitives - https://iquilezles.org/articles/distfunctions/
fn sd_sphere(point: vec3f, radius: f32) -> f32 {
    return length(point) - radius;
}

fn sd_cuboid(point: vec3f, extent: vec3f) -> f32 {
    let q = abs(point) - extent;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sd_round_cuboid(point: vec3f, extent: vec3f, radius: f32) -> f32 {
    let q = abs(point) - extent + radius;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0) - radius;
}

fn sd_torus(point: vec3f, t: vec2f) -> f32 {
    let q = vec2f(length(point.xz) - t.x, point.y);
    return length(q) - t.y;
}

fn sd_capsule(point: vec3f, point_a: vec3f, point_b: vec3f, radius: f32) -> f32 {
    let vec_pa = point - point_a;
    let vec_ba = point_b - point_a;
    let h = clamp(dot(vec_pa, vec_ba) / dot(vec_ba, vec_ba), 0.0, 1.0);
    return length(vec_pa - vec_ba * h) - radius;
}

/// Scene composition.
fn composition(point: vec3f) -> f32 {
    let len = arrayLength(&inputs);
    var dist = sdf_camera.far_plane;

    // TODO: Implement acceleration structure (BVH?) to prevent looping over
    // the entire transform buffer.
    // TODO: Support different SDF primitives.
    for (var i = 0u; i < len; i++) {
        let input = inputs[i];
        let sample_point = ((vec4f(point, 1.0) * input.local_from_world).xyz) / input.scale;

        var sdf_dist = dist;
        switch input.primitive_type {
            default { }
            case SPHERE {
                let radius = spheres[input.primitive_index].radius;
                sdf_dist = sd_sphere(sample_point, radius);
            }
            case CUBOID {
                let extents = cuboids[input.primitive_index].extents;
                sdf_dist = sd_cuboid(sample_point, extents);
            }
            case ROUND_CUBOID {
                let round_cuboid = round_cuboids[input.primitive_index];
                sdf_dist = sd_round_cuboid(sample_point, round_cuboid.extents, round_cuboid.radius);
            }
            case CAPSULE {
                let capsule = capsules[input.primitive_index];
                sdf_dist = sd_capsule(sample_point, capsule.point_a, capsule.point_b, capsule.radius);
            }
        };

        dist = min(dist, sdf_dist * input.scale);
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
