struct Transform {
    mtx: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> r_proj: Transform;
@group(0) @binding(1) var<uniform> r_view: Transform;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
  @location(0) center: vec3<f32>,
  @location(1) radius: vec2<f32>,
  @location(2) color: vec4<f32>,
  @location(3) offset: vec2<f32>,
) -> VertexOutput {
  var screen_center: vec4<f32> = r_proj.mtx * r_view.mtx * vec4<f32>(center, 1.0);
  var screen_offset: vec2<f32> = radius * offset;
  return VertexOutput(
    screen_center + screen_center.w * vec4<f32>(screen_offset, 0.0, 0.0),
    color,
  );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return in.color;
}
