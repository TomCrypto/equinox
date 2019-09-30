#include <common.glsl>

vec2 multiplyComplex (vec2 a, vec2 b) {
    return vec2(a[0] * b[0] - a[1] * b[1], a[1] * b[0] + a[0] * b[1]);
}

uniform sampler2D r_spectrum_input;
uniform sampler2D g_spectrum_input;
uniform sampler2D b_spectrum_input;

uniform sampler2D r_aperture_input;
uniform sampler2D g_aperture_input;
uniform sampler2D b_aperture_input;

layout(location = 0) out vec4 r_spectrum_output;
layout(location = 1) out vec4 g_spectrum_output;
layout(location = 2) out vec4 b_spectrum_output;

void main(void){
    vec2 r_aperture = texelFetch(r_aperture_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;
    vec2 g_aperture = texelFetch(g_aperture_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;
    vec2 b_aperture = texelFetch(b_aperture_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;

    vec2 r_value = texelFetch(r_spectrum_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;
    vec2 g_value = texelFetch(g_spectrum_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;
    vec2 b_value = texelFetch(b_spectrum_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;

    r_value = multiplyComplex(r_value, r_aperture);
    g_value = multiplyComplex(g_value, g_aperture);
    b_value = multiplyComplex(b_value, b_aperture);

    r_spectrum_output = vec4(r_value, 0.0, 0.0);
    g_spectrum_output = vec4(g_value, 0.0, 0.0);
    b_spectrum_output = vec4(b_value, 0.0, 0.0);
}
