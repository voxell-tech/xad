#import bevy_pbr::mesh_view_bindings::globals
#import bevy_pbr::utils::PI

struct SdfMaterial {
    camera_pos: vec4f,
};

@group(2) @binding(0)
var<uniform> material: SdfMaterial;

// define SDFs here
// https://iquilezles.org/articles/distfunctions/
fn sdRoundCone(p: vec3f, r1: f32, r2: f32, h: f32) -> f32 {
    let b = (r1 - r2) / h;
    let a = sqrt(1.0 - b * b);

    let q = vec2f(length(p.xz), p.y);
    let k = dot(q, vec2f(-b, a));
    
    if (k < 0.0) { 
        return length(q) - r1; 
    }
    if (k > a * h) { 
        return length(q - vec2f(0.0, h)) - r2; 
    }
    
    return dot(q, vec2f(a, b)) - r1;
}
fn sdRoundBox(p: vec3f, b: vec3f, r: f32) -> f32 {
    let q = abs(p) - b + r;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

// combine shapes
fn map(p: vec3f) -> f32 {
    return sdRoundBox(p, vec3f(1.0, 1.0, 1.0), 0.1);
    // return sdRoundCone(p, 0.8, 0.4, 2.0);
}

// main fragment shader
// wgsl moment D:
@fragment
fn fragment(
    @location(0) world_position: vec4f,
    @location(2) uv: vec2f,
) -> @location(0) vec4f {
    let ray_dir = normalize(world_position.xyz - material.camera_pos.xyz);
    var t = 0.0;
    
    // step size is here
    for (var i = 0; i < 256; i++) {
        let p = material.camera_pos.xyz + ray_dir * t;
        let d = map(p);
        
        if d < 0.001 {
            // visualize distance depth
            return vec4f(vec3f(1.0 - (t / 10.0)), 1.0);
        }
        
        t += d;
        if t > 20.0 { break; }
    }

    return vec4f(1.0, 0.0, 1.0, 1.0); // background
}