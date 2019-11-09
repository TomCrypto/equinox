#include <common.glsl>

layout (location = 0) out vec4 li_range;

uniform sampler2D ld_count_tex;
uniform sampler2D li_count_tex;

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
    float pass_count;
    float photons_for_pass;
    float total_photons;
    float grid_cell_size;
    uint hash_cell_cols;
    uint hash_cell_rows;
    uint hash_cell_col_bits;
    float alpha;
} integrator;

void extract_pass_pixel_data(ivec2 coords, out vec3 li, out float count, out float photons) {
    vec4 li_data = texelFetch(li_count_tex, coords, 0);

    li = li_data.rgb;
    count = texelFetch(ld_count_tex, coords, 0).w;
    photons = li_data.w;
}

void main() {
    vec3 li;
    float count, photons;

    extract_pass_pixel_data(ivec2(gl_FragCoord.xy - 0.5), li, count, photons);

    float ratio = 1.0;

    if (count != 0.0) {
        ratio = count / (count + photons * (1.0 - integrator.alpha));
    }

    li_range = vec4(li, ratio);
}
