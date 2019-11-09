#include <common.glsl>

layout (location = 0) out vec3 radiance;

uniform sampler2D ld_count_tex;
uniform sampler2D li_range_tex;

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

void extract_estimates(ivec2 coords, out vec3 ld, out vec3 li, out float range) {
    vec4 li_data = texelFetch(li_range_tex, coords, 0);

    ld = texelFetch(ld_count_tex, coords, 0).rgb / integrator.pass_count;
    li = li_data.rgb;
    range = li_data.w;
}

void main() {
    vec3 ld, li;
    float range;

    extract_estimates(ivec2(gl_FragCoord.xy - 0.5), ld, li, range);
    radiance = ld + li / (integrator.total_photons * M_PI * range);
}
