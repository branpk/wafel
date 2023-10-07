@group(0) @binding(0) var<uniform> r_proj: mat4x4<f32>;
@group(0) @binding(1) var<uniform> r_view: mat4x4<f32>;

struct VertexInput {
    @location(0) center: vec4<f32>,
    @location(1) radius: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) decal_amount: f32,
    @location(4) offset: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) decal_amount: f32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out = VertexOutput();
    let screen_center = r_proj * r_view * in.center;
    let screen_offset = in.radius * in.offset;
    out.position = screen_center + screen_center.w * vec4<f32>(screen_offset, 0.0, 0.0);
    out.color = in.color;
    out.decal_amount = in.decal_amount;
    return out;
}

struct FragmentOutput {
    @builtin(frag_depth) frag_depth: f32,
    @location(0) color: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out = FragmentOutput();
    out.frag_depth = in.position.z - in.decal_amount;
    out.color = in.color;
    return out;
}
