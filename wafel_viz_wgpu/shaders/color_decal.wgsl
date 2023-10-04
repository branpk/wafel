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

struct FragmentOutput {
    @builtin(frag_depth) frag_depth: f32,
    @location(0) color: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out = FragmentOutput();
    out.frag_depth = in.position.z - 0.0002;
    out.color = in.color;
    return out;
}
