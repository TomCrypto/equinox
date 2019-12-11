layout(location = 0) out vec2 filter_tile_r;
layout(location = 1) out vec2 filter_tile_g;
layout(location = 2) out vec2 filter_tile_b;

uniform sampler2D filter_tex;

uniform ivec2 tile_offset;
uniform ivec2 tile_size;

void main() {
    int padding = tile_size.x / 4;

    ivec2 coords = ivec2(gl_FragCoord.xy - 0.5);

    if ((coords.x > padding && coords.x <= 3 * padding) || (coords.y > padding && coords.y <= 3 * padding)) {
        return; // zero padding...
    }

    coords += tile_size / 4 - 1;
    coords %= tile_size;

    vec3 value = texelFetch(filter_tex, coords + tile_offset, 0).rgb;

    filter_tile_r = vec2(value.r, 0.0);
    filter_tile_g = vec2(value.g, 0.0);
    filter_tile_b = vec2(value.b, 0.0);
}
