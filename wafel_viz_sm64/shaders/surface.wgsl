@group(0) @binding(0) var<uniform> r_proj: mat4x4<f32>;
@group(0) @binding(1) var<uniform> r_view: mat4x4<f32>;

struct VertexData {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) bary: vec3<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexData) -> VertexOutput {
    var bary_vertices = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );

    var out = VertexOutput();
    out.position = r_proj * r_view * in.pos;
    out.bary = bary_vertices[in.vertex_index % 3u];
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let b = 0.8;

    var inner_bary = vec3<f32>(
        (b + 1.0) * in.bary.x + (b - 1.0) * in.bary.y + (b - 1.0) * in.bary.z,
        (b - 1.0) * in.bary.x + (b + 1.0) * in.bary.y + (b - 1.0) * in.bary.z,
        (b - 1.0) * in.bary.x + (b - 1.0) * in.bary.y + (b + 1.0) * in.bary.z,
    );
    inner_bary = inner_bary * 1.0 / (3.0 * b - 1.0);

    var t = length(vec3<f32>(
        clamp(-inner_bary.x, 0.0, 1.0),
        clamp(-inner_bary.y, 0.0, 1.0),
        clamp(-inner_bary.z, 0.0, 1.0),
    ));
    t = t * -(3.0 * b - 1.0)/(b - 1.0) * 0.7;

    let inner_color: vec4<f32> = vec4<f32>(in.color.rgb * 0.5, in.color.a);
    let outer_color: vec4<f32> = vec4<f32>(in.color.rgb * 0.8, max(in.color.a, 0.8));
    return mix(inner_color, outer_color, t);
}
