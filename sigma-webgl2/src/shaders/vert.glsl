#version 300 es

out vec2 clip;

void main() {
    gl_Position.x = float((gl_VertexID & 1) << 2) - 1.0;
    gl_Position.y = float((gl_VertexID & 2) << 1) - 1.0;
    gl_Position.z = 0.0;
    gl_Position.w = 1.0;

    clip = gl_Position.xy * 0.5 + 0.5;
}
