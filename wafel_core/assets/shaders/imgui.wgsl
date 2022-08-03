struct Transform {
    mtx: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> r_proj: Transform;

@group(1) @binding(0) var r_sampler: sampler;
@group(1) @binding(1) var r_texture: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) pos: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
) -> VertexOutput {
    return VertexOutput(
        r_proj.mtx * vec4<f32>(pos, 0.0, 1.0),
        tex_coord,
        color,
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color * textureSample(r_texture, r_sampler, in.tex_coord);
}
