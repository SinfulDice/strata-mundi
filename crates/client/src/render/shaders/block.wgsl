struct Camera {
    view_proj: mat4x4<f32>,
};

// Mesh origin relative to the camera (camera-relative rendering), set per draw.
var<immediate> mesh_offset: vec4<f32>;

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var block_textures: texture_2d_array<f32>;
@group(1) @binding(1) var block_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) layer: u32,
    @location(3) shade: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) shade: f32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = camera.view_proj * vec4<f32>(input.position + mesh_offset.xyz, 1.0);
    out.uv = input.uv;
    out.layer = input.layer;
    out.shade = input.shade;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, input.uv, input.layer);
    return vec4<f32>(texel.rgb * input.shade, texel.a);
}
