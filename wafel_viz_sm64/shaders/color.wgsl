@group(0) @binding(0) var<uniform> r_proj: mat4x4<f32>;
@group(0) @binding(1) var<uniform> r_view: mat4x4<f32>;

struct VertexData {
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @location(0) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(in: VertexData) -> VertexOutput {
    var out = VertexOutput();
    out.color = in.color;
    out.position = r_proj * r_view * in.pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
