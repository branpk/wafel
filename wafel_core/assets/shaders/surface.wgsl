struct Transform {
    matrix: mat4x4<f32>;
};

[[group(0), binding(0)]] var<uniform> r_proj: Transform;
[[group(0), binding(1)]] var<uniform> r_view: Transform;

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] bary: vec3<f32>;
    [[location(1)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(
    [[builtin(vertex_index)]] vertex_index: u32,
    [[location(0)]] pos: vec3<f32>,
    [[location(1)]] color: vec4<f32>,
) -> VertexOutput {
    var bary_vertices: array<vec3<f32>, 3> = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );
    return VertexOutput(
        r_proj.matrix * r_view.matrix * vec4<f32>(pos, 1.0),
        bary_vertices[vertex_index % 3u],
        color,
    );
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var b: f32 = 0.8;

    var inner_bary: vec3<f32> = vec3<f32>(
        (b + 1.0) * in.bary.x + (b - 1.0) * in.bary.y + (b - 1.0) * in.bary.z,
        (b - 1.0) * in.bary.x + (b + 1.0) * in.bary.y + (b - 1.0) * in.bary.z,
        (b - 1.0) * in.bary.x + (b - 1.0) * in.bary.y + (b + 1.0) * in.bary.z,
    );
    inner_bary = inner_bary * 1.0 / (3.0 * b - 1.0);

    var t: f32 = length(vec3<f32>(
        clamp(-inner_bary.x, 0.0, 1.0),
        clamp(-inner_bary.y, 0.0, 1.0),
        clamp(-inner_bary.z, 0.0, 1.0),
    ));
    t = t * -(3.0 * b - 1.0)/(b - 1.0) * 0.7;

    var inner_color: vec4<f32> = vec4<f32>(in.color.rgb * 0.5, in.color.a);
    var outer_color: vec4<f32> = vec4<f32>(in.color.rgb * 0.8, max(in.color.a, 0.8));
    return mix(inner_color, outer_color, vec4<f32>(t));
}
