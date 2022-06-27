struct VertexData {
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @location(0) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(vertex: VertexData) -> VertexOutput {
    var output = VertexOutput();
    output.color = vertex.color;
    output.position = vertex.pos;
    return output;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color;
}
