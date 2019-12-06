layout(location = 0) out vec2 signal_tile_r;
layout(location = 1) out vec2 signal_tile_g;
layout(location = 2) out vec2 signal_tile_b;

uniform sampler2D signal;

void main() {
    vec4 value = texelFetch(signal, ivec2(gl_FragCoord.xy - 0.5) + tile_offset, 0);

    signal_tile_r = vec2(value.r / value.w, 0.0);
    signal_tile_g = vec2(value.g / value.w, 0.0);
    signal_tile_b = vec2(value.b / value.w, 0.0);
}
