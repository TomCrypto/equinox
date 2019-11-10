#include <common.glsl>

#include <integrator.glsl>

layout (location = 0) out vec3 radiance;

uniform sampler2D ld_count_tex;
uniform sampler2D li_range_tex;

void unpack_estimates(ivec2 coords, out vec3 ld, out vec3 li, out float range) {
    vec4 li_data = texelFetch(li_range_tex, coords, 0);

    ld = texelFetch(ld_count_tex, coords, 0).rgb / float(integrator.current_pass);
    li = li_data.rgb;
    range = min(li_data.w, pow(integrator.cell_size * 0.5, 2.0));
}

void main() {
    vec3 ld, li;
    float range;

    unpack_estimates(ivec2(gl_FragCoord.xy - 0.5), ld, li, range);
    radiance = ld + li / (integrator.photon_count * M_PI * range);
}
