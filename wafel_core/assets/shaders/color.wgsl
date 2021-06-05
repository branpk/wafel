[[block]]
struct Transform {
    matrix: mat4x4<f32>;
};

[[group(0), binding(0)]] var r_proj: Transform;
[[group(0), binding(1)]] var r_view: Transform;

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn main(
    [[location(0)]] pos: vec3<f32>,
    [[location(1)]] color: vec4<f32>,
) -> VertexOutput {
    return VertexOutput(
        r_proj.matrix * r_view.matrix * vec4<f32>(pos, 1.0),
        color,
    );
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}
