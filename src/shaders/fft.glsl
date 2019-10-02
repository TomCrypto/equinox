#include <common.glsl>

uniform sampler2D r_spectrum_input;
uniform sampler2D g_spectrum_input;
uniform sampler2D b_spectrum_input;

layout(location = 0) out vec2 r_spectrum_output;
layout(location = 1) out vec2 g_spectrum_output;
layout(location = 2) out vec2 b_spectrum_output;

layout (std140) uniform FFT {
    int transformSize;
    int subtransformSize;
    float horizontal;
    float direction;
} fft;

void main(void){
    int index;

    if (fft.horizontal == 1.0) {
        index = int(gl_FragCoord.x - 0.5);
    } else {
        index = int(gl_FragCoord.y - 0.5);
    }

    int evenIndex = ((index / fft.subtransformSize) * (fft.subtransformSize / 2) + (index % (fft.subtransformSize / 2))) % fft.transformSize;

    vec2 r_even, g_even, b_even;
    vec2 r_odd, g_odd, b_odd;

    if (fft.horizontal == 1.0) {
        r_even = texelFetch(r_spectrum_input, ivec2(evenIndex, int(gl_FragCoord.y - 0.5)), 0).rg;
        g_even = texelFetch(g_spectrum_input, ivec2(evenIndex, int(gl_FragCoord.y - 0.5)), 0).rg;
        b_even = texelFetch(b_spectrum_input, ivec2(evenIndex, int(gl_FragCoord.y - 0.5)), 0).rg;
        
        r_odd = texelFetch(r_spectrum_input, ivec2((evenIndex + fft.transformSize / 2) % fft.transformSize, int(gl_FragCoord.y - 0.5)), 0).rg;
        g_odd = texelFetch(g_spectrum_input, ivec2((evenIndex + fft.transformSize / 2) % fft.transformSize, int(gl_FragCoord.y - 0.5)), 0).rg;
        b_odd = texelFetch(b_spectrum_input, ivec2((evenIndex + fft.transformSize / 2) % fft.transformSize, int(gl_FragCoord.y - 0.5)), 0).rg;
    } else {
        r_even = texelFetch(r_spectrum_input, ivec2(int(gl_FragCoord.x - 0.5), evenIndex), 0).rg;
        g_even = texelFetch(g_spectrum_input, ivec2(int(gl_FragCoord.x - 0.5), evenIndex), 0).rg;
        b_even = texelFetch(b_spectrum_input, ivec2(int(gl_FragCoord.x - 0.5), evenIndex), 0).rg;
        
        r_odd = texelFetch(r_spectrum_input, ivec2(int(gl_FragCoord.x - 0.5), (evenIndex + fft.transformSize / 2) % fft.transformSize), 0).rg;
        g_odd = texelFetch(g_spectrum_input, ivec2(int(gl_FragCoord.x - 0.5), (evenIndex + fft.transformSize / 2) % fft.transformSize), 0).rg;
        b_odd = texelFetch(b_spectrum_input, ivec2(int(gl_FragCoord.x - 0.5), (evenIndex + fft.transformSize / 2) % fft.transformSize), 0).rg;
    }

    float twiddleArgument1D = -fft.direction * M_2PI * (float(index % fft.subtransformSize) / float(fft.subtransformSize));
    vec2 twiddle1D = vec2(cos(twiddleArgument1D), sin(twiddleArgument1D));

    r_spectrum_output = r_even + complex_mul(twiddle1D, r_odd);
    g_spectrum_output = g_even + complex_mul(twiddle1D, g_odd);
    b_spectrum_output = b_even + complex_mul(twiddle1D, b_odd);
}
