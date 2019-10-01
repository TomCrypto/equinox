out vec2 screen_coords;

void main() {
    gl_Position = vec4(
        float((gl_VertexID & 1) << 2) - 1.0,
        float((gl_VertexID & 2) << 1) - 1.0,
        0.0,
        1.0
    );

    screen_coords = vec2(gl_Position.xy * 0.5 + 0.5);
}
