void main() {
    gl_Position = vec4(
        float((gl_VertexID & 1) << 2) - 1.0,
        float((gl_VertexID & 2) << 1) - 1.0,
        0.0,
        1.0
    );
}
