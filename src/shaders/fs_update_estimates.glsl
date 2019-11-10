#include <integrator.glsl>

layout (location = 0) out vec4 li_range;

uniform sampler2D ld_count_tex;
uniform sampler2D li_count_tex;

void unpack_pixel_pass_state(ivec2 coords, out vec3 li, out float count, out float photons) {
    vec4 ld_data = texelFetch(ld_count_tex, coords, 0);
    vec4 li_data = texelFetch(li_count_tex, coords, 0);

    li = li_data.rgb;
    count = ld_data.a;
    photons = li_data.a;
}

void main() {
    // This factor determines how quickly the pixel radius decreases
    float K = (1.0 - integrator.sppm_alpha) / integrator.sppm_alpha;

    vec3 li;
    float count, photons;

    unpack_pixel_pass_state(ivec2(gl_FragCoord.xy - 0.5), li, count, photons);
    li_range = vec4(li, (count == 0.0) ? 1.0 : count / (count + photons * K));
}
