// This is a basic SDF shader, feel free to experiment.

#import bevy_pbr::forward_io::VertexOutput

struct SdfMaterial {
    camera_pos: vec4f,
};

@group(0) @binding(0)
var<uniform> material: SdfMaterial;

// Define SDFs here.
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

// calculate normal at a point
fn calculate_normal(p: vec3f) -> vec3f {
    let e = vec2f(0.001, 0.0); // differentials!!!
    return normalize(vec3f(
        map(p + e.xyy) - map(p - e.xyy),
        map(p + e.yxy) - map(p - e.yxy),
        map(p + e.yyx) - map(p - e.yyx)
    ));
}

// Main fragment shader.
@fragment
fn fragment(
    mesh: VertexOutput,
    // @location(0) world_position: vec4f,
) -> @location(0) vec4f {
    let ray_dir = normalize(mesh.world_position.xyz - material.camera_pos.xyz);
    // return vec4f(mesh.uv, 0.0, 0.0);
    // return vec4f(ray_dir, 0.0);
    var t = 0.0;
    var dist = 0.0;
    
    // step size is here
    for (var i = 0; i < 256; i++) {
        let p = material.camera_pos.xyz + ray_dir * t;
        let d = map(p);
        
        if d < 0.001 {
            // visualize distance depth
            // return vec4f(vec3f(1.0 - (t / 10.0)), 1.0);

            let normal = calculate_normal(p);
            // let normal = normal * 0.5 + 0.5;
            return vec4f(abs(normal), 1.0);
        }
        
        t += d;
        dist = d;
        if t > 20.0 { break; }
    }

    return vec4f(dist / 20.0);
    return vec4f(0.1, 0.1, 0.1, 1.0); // background
}
