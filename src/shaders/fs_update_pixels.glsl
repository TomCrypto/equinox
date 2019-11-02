uniform sampler2D old_photon_count_tex;
uniform sampler2D old_photon_data_tex;
uniform sampler2D new_photon_data_tex;

layout(location = 0) out float photon_count;
layout(location = 1) out vec4 photon_data;
layout(location = 2) out vec3 photon_radiance;

void main() {
    ivec2 coords = ivec2(gl_FragCoord.xy - 0.5);

    float old_photon_count = texelFetch(old_photon_count_tex, coords, 0).r;
    vec4 old_photon_data = texelFetch(old_photon_data_tex, coords, 0).rgba;
    vec4 new_photon_data = texelFetch(new_photon_data_tex, coords, 0).rgba;

    // update rule 1
    photon_count = old_photon_count + 0.666 * new_photon_data.w;
    // update rule 2
    if (photon_count == 0.0) {
        photon_data.w = old_photon_data.w;
    } else {
        photon_data.w = old_photon_data.w * sqrt(photon_count / (old_photon_count + new_photon_data.w));
    }
    
    // update rule 3
    photon_data.rgb = (old_photon_data.rgb + new_photon_data.rgb) * pow(photon_data.w / old_photon_data.w, 2.0);

    if (photon_count == 0.0) {
        photon_radiance = photon_data.rgb;
    } else {
        photon_radiance = photon_data.rgb / photon_count;
    }
}
