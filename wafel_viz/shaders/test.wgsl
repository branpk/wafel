struct VertexData {
    @location(0) pos: vec4<f32>,
}

@vertex
fn vs_main(vertex: VertexData) -> @builtin(position) vec4<f32> {
    return vertex.pos;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
