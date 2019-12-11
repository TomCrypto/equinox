layout(location = 0) out vec4 canvas;

uniform sampler2D render;

void main() {
    canvas = texelFetch(render, ivec2(gl_FragCoord.xy - 0.5), 0);
}
