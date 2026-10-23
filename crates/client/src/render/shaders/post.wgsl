// Final pass: draws the 3D image (possibly rendered smaller or larger than the window)
// over the whole frame, through an optional colour-blindness filter.

struct Post {
    // x: filter (0 none, 1 protanopia, 2 deuteranopia, 3 tritanopia).
    mode: vec4<u32>,
};

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> post: Post;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One triangle covering the screen.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOutput;
    out.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

// Daltonization: simulate what the eye misses, then move that difference into colours
// it can tell apart (Fidaner, Lin and Ozguven's method).
fn daltonize(rgb: vec3<f32>, mode: u32) -> vec3<f32> {
    let to_lms = mat3x3<f32>(
        vec3<f32>(17.8824, 3.45565, 0.0299566),
        vec3<f32>(43.5161, 27.1554, 0.184309),
        vec3<f32>(4.11935, 3.86714, 1.46709),
    );
    let to_rgb = mat3x3<f32>(
        vec3<f32>(0.0809444479, -0.0102485335, -0.000365296938),
        vec3<f32>(-0.130504409, 0.0540193266, -0.00412161469),
        vec3<f32>(0.116721066, -0.113614708, 0.693511405),
    );
    let lms = to_lms * rgb;
    var seen = lms;
    if mode == 1u {
        seen.x = 2.02344 * lms.y - 2.52581 * lms.z;
    } else if mode == 2u {
        seen.y = 0.494207 * lms.x + 1.24827 * lms.z;
    } else {
        seen.z = -0.395913 * lms.x + 0.801109 * lms.y;
    }
    let error = rgb - to_rgb * seen;
    let shift = vec3<f32>(0.0, 0.7 * error.r + error.g, 0.7 * error.r + error.b);
    return clamp(rgb + shift, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(scene, scene_sampler, input.uv).rgb;
    if post.mode.x == 0u {
        return vec4<f32>(color, 1.0);
    }
    return vec4<f32>(daltonize(color, post.mode.x), 1.0);
}
