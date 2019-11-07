#include <common.glsl>

uniform sampler2D old_photon_count_tex;
uniform sampler2D old_photon_data_tex;
uniform sampler2D new_photon_data_tex;

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
} globals;

layout(location = 0) out vec4 photon_direct_and_count;
layout(location = 1) out vec4 photon_data;
layout(location = 2) out vec3 photon_radiance;

void main() {
    ivec2 coords = ivec2(gl_FragCoord.xy - 0.5);

    vec4 old_photon_other = texelFetch(old_photon_count_tex, coords, 0).rgba;
    vec4 old_photon_data = texelFetch(old_photon_data_tex, coords, 0).rgba;
    vec4 new_photon_data = texelFetch(new_photon_data_tex, coords, 0).rgba;

    vec3 old_photon_direct = old_photon_other.rgb;
    float old_photon_count = old_photon_other.w;

    if (new_photon_data.w == 0.0) {
        // treat as direct lighting
        photon_direct_and_count.w = old_photon_count;
        photon_direct_and_count.rgb = old_photon_direct + new_photon_data.rgb;

        photon_data = old_photon_data;
    } else {
        // treat as photon contributions
        float photon_count = old_photon_count + globals.alpha * new_photon_data.w;

        float ratio = photon_count / (old_photon_count + new_photon_data.w);

        photon_direct_and_count.rgb = old_photon_direct;
        photon_direct_and_count.w = photon_count;
        photon_data.w = old_photon_data.w * sqrt(ratio);
        photon_data.rgb = (old_photon_data.rgb + new_photon_data.rgb) * ratio;
    }

    photon_radiance = photon_direct_and_count.rgb / (globals.pass_count)
                    + photon_data.rgb / (globals.total_photons * M_PI * pow(photon_data.w, 2.0));
}
