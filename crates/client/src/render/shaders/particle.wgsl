// Particles: one camera-facing square per instance, a piece of a block texture, lit by
// the light of its cell and fogged like blocks.

// Camera right and up, in the world.
struct Billboard {
    right: vec4<f32>,
    up: vec4<f32>,
};

var<immediate> billboard: Billboard;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) @interpolate(flat) color: vec3<f32>,
    @location(3) relative: vec3<f32>,
    @location(4) @interpolate(flat) light: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) corner: u32,
    @location(0) centre: vec3<f32>,
    @location(1) size: f32,
    @location(2) color: vec3<f32>,
    @location(3) layer: u32,
    @location(4) piece: vec3<f32>,
    @location(5) light: vec2<f32>,
) -> VertexOutput {
    let x = f32(corner & 1u);
    let y = f32(corner >> 1u);
    let spread = billboard.right.xyz * (x - 0.5) + billboard.up.xyz * (y - 0.5);
    let position = centre + spread * size;
    var out: VertexOutput;
    out.clip = camera.view_proj * vec4<f32>(position, 1.0);
    out.uv = piece.xy + vec2<f32>(x, 1.0 - y) * piece.z;
    out.layer = layer;
    out.color = color;
    out.relative = position;
    out.light = light;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, input.uv, input.layer);
    if texel.a < 0.5 {
        discard;
    }
    let color = texel.rgb * input.color * light_color(input.light.x, input.light.y);
    return vec4<f32>(fogged(color, input.relative), 1.0);
}
