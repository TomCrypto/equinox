uniform sampler2D old_photon_count_tex;
uniform sampler2D old_photon_data_tex;
uniform sampler2D new_photon_data_tex;

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
    float pass_count;
} globals;

layout(location = 0) out float photon_count;
layout(location = 1) out vec4 photon_data;
layout(location = 2) out vec3 photon_radiance;

#define ALPHA 0.95

void main() {
    ivec2 coords = ivec2(gl_FragCoord.xy - 0.5);

    float old_photon_count = texelFetch(old_photon_count_tex, coords, 0).r;
    vec4 old_photon_data = texelFetch(old_photon_data_tex, coords, 0).rgba;
    vec4 new_photon_data = texelFetch(new_photon_data_tex, coords, 0).rgba;

    // update rules
    if (new_photon_data.w == 0.0) {
        photon_count = old_photon_count;
        photon_data = old_photon_data + new_photon_data;
    } else {
        photon_count = old_photon_count + ALPHA * new_photon_data.w;

        float ratio = photon_count / (old_photon_count + new_photon_data.w);
        
        photon_data.w = old_photon_data.w * sqrt(ratio);
        photon_data.rgb = (old_photon_data.rgb + new_photon_data.rgb) * ratio;
    }

    photon_radiance = photon_data.rgb / (1.0 * globals.pass_count);
}