#include <common.glsl>

uniform sampler2D r_spectrum_input;
uniform sampler2D g_spectrum_input;
uniform sampler2D b_spectrum_input;

uniform sampler2D r_aperture_input;
uniform sampler2D g_aperture_input;
uniform sampler2D b_aperture_input;

layout(location = 0) out vec2 r_spectrum_output;
layout(location = 1) out vec2 g_spectrum_output;
layout(location = 2) out vec2 b_spectrum_output;

void main(void){
    vec2 r_aperture = texelFetch(r_aperture_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;
    vec2 g_aperture = texelFetch(g_aperture_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;
    vec2 b_aperture = texelFetch(b_aperture_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;

    vec2 r_value = texelFetch(r_spectrum_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;
    vec2 g_value = texelFetch(g_spectrum_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;
    vec2 b_value = texelFetch(b_spectrum_input, ivec2(gl_FragCoord.xy - 0.5), 0).rg;

    r_spectrum_output = complex_mul(r_value, r_aperture);
    g_spectrum_output = complex_mul(g_value, g_aperture);
    b_spectrum_output = complex_mul(b_value, b_aperture);
}
