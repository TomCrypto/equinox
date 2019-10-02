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
    int subtransformShift;
    int subtransformMask;
    float horizontal;
    float direction;
} fft;

// TODO: for the forward FFT, we must operate in reverse order (subtransformSize goes from transformSize down to 2!!)

void forward_fft() {
    int i = int(((fft.horizontal == 1.0) ? gl_FragCoord.x : gl_FragCoord.y) - 0.5);
    
    int j = i & fft.subtransformMask;

    if (2 * j < fft.subtransformSize) {
        if (fft.horizontal == 1.0) {
            int z = int(gl_FragCoord.y - 0.5);

            vec2 r_t = texelFetch(r_spectrum_input, ivec2(i, z), 0).rg;
            vec2 g_t = texelFetch(g_spectrum_input, ivec2(i, z), 0).rg;
            vec2 b_t = texelFetch(b_spectrum_input, ivec2(i, z), 0).rg;

            vec2 r_u = texelFetch(r_spectrum_input, ivec2(i + fft.subtransformSize / 2, z), 0).rg;
            vec2 g_u = texelFetch(g_spectrum_input, ivec2(i + fft.subtransformSize / 2, z), 0).rg;
            vec2 b_u = texelFetch(b_spectrum_input, ivec2(i + fft.subtransformSize / 2, z), 0).rg;

            r_spectrum_output = r_t + r_u;
            g_spectrum_output = g_t + g_u;
            b_spectrum_output = b_t + b_u;
        } else {
            int z = int(gl_FragCoord.x - 0.5);

            vec2 r_t = texelFetch(r_spectrum_input, ivec2(z, i), 0).rg;
            vec2 g_t = texelFetch(g_spectrum_input, ivec2(z, i), 0).rg;
            vec2 b_t = texelFetch(b_spectrum_input, ivec2(z, i), 0).rg;

            vec2 r_u = texelFetch(r_spectrum_input, ivec2(z, i + fft.subtransformSize / 2), 0).rg;
            vec2 g_u = texelFetch(g_spectrum_input, ivec2(z, i + fft.subtransformSize / 2), 0).rg;
            vec2 b_u = texelFetch(b_spectrum_input, ivec2(z, i + fft.subtransformSize / 2), 0).rg;

            r_spectrum_output = r_t + r_u;
            g_spectrum_output = g_t + g_u;
            b_spectrum_output = b_t + b_u;
        }
    } else {
        float twiddle = M_2PI * float(j) / float(fft.subtransformSize);

        vec2 w = vec2(-cos(twiddle), sin(twiddle));

        if (fft.horizontal == 1.0) {
            int z = int(gl_FragCoord.y - 0.5);

            vec2 r_t = texelFetch(r_spectrum_input, ivec2(i - fft.subtransformSize / 2, z), 0).rg;
            vec2 g_t = texelFetch(g_spectrum_input, ivec2(i - fft.subtransformSize / 2, z), 0).rg;
            vec2 b_t = texelFetch(b_spectrum_input, ivec2(i - fft.subtransformSize / 2, z), 0).rg;

            vec2 r_u = texelFetch(r_spectrum_input, ivec2(i, z), 0).rg;
            vec2 g_u = texelFetch(g_spectrum_input, ivec2(i, z), 0).rg;
            vec2 b_u = texelFetch(b_spectrum_input, ivec2(i, z), 0).rg;

            r_spectrum_output = complex_mul(w, r_t - r_u);
            g_spectrum_output = complex_mul(w, g_t - g_u);
            b_spectrum_output = complex_mul(w, b_t - b_u);
        } else {
            int z = int(gl_FragCoord.x - 0.5);

            vec2 r_t = texelFetch(r_spectrum_input, ivec2(z, i - fft.subtransformSize / 2), 0).rg;
            vec2 g_t = texelFetch(g_spectrum_input, ivec2(z, i - fft.subtransformSize / 2), 0).rg;
            vec2 b_t = texelFetch(b_spectrum_input, ivec2(z, i - fft.subtransformSize / 2), 0).rg;

            vec2 r_u = texelFetch(r_spectrum_input, ivec2(z, i), 0).rg;
            vec2 g_u = texelFetch(g_spectrum_input, ivec2(z, i), 0).rg;
            vec2 b_u = texelFetch(b_spectrum_input, ivec2(z, i), 0).rg;

            r_spectrum_output = complex_mul(w, r_t - r_u);
            g_spectrum_output = complex_mul(w, g_t - g_u);
            b_spectrum_output = complex_mul(w, b_t - b_u);
        }
    }
}

void inverse_fft() {
    int i = int(((fft.horizontal == 1.0) ? gl_FragCoord.x : gl_FragCoord.y) - 0.5);

    int j = i & fft.subtransformMask;

    float twiddle = -M_2PI * float(j) / float(fft.subtransformSize);

    vec2 w = vec2(cos(twiddle), -sin(twiddle));

    int ti = i - (j / (fft.subtransformSize / 2)) * (fft.subtransformSize / 2);
    int ui = ti + fft.subtransformSize / 2;

    if (fft.horizontal == 1.0) {
        int z = int(gl_FragCoord.y - 0.5);

        vec2 r_t = texelFetch(r_spectrum_input, ivec2(ti, z), 0).rg;
        vec2 g_t = texelFetch(g_spectrum_input, ivec2(ti, z), 0).rg;
        vec2 b_t = texelFetch(b_spectrum_input, ivec2(ti, z), 0).rg;

        vec2 r_u = texelFetch(r_spectrum_input, ivec2(ui, z), 0).rg;
        vec2 g_u = texelFetch(g_spectrum_input, ivec2(ui, z), 0).rg;
        vec2 b_u = texelFetch(b_spectrum_input, ivec2(ui, z), 0).rg;

        r_spectrum_output = r_t + complex_mul(w, r_u);
        g_spectrum_output = g_t + complex_mul(w, g_u);
        b_spectrum_output = b_t + complex_mul(w, b_u);
    } else {
        int z = int(gl_FragCoord.x - 0.5);

        vec2 r_t = texelFetch(r_spectrum_input, ivec2(z, ti), 0).rg;
        vec2 g_t = texelFetch(g_spectrum_input, ivec2(z, ti), 0).rg;
        vec2 b_t = texelFetch(b_spectrum_input, ivec2(z, ti), 0).rg;

        vec2 r_u = texelFetch(r_spectrum_input, ivec2(z, ui), 0).rg;
        vec2 g_u = texelFetch(g_spectrum_input, ivec2(z, ui), 0).rg;
        vec2 b_u = texelFetch(b_spectrum_input, ivec2(z, ui), 0).rg;

        r_spectrum_output = r_t + complex_mul(w, r_u);
        g_spectrum_output = g_t + complex_mul(w, g_u);
        b_spectrum_output = b_t + complex_mul(w, b_u);
    }
}

void main(void){
    if (fft.direction == 1.0) {
        forward_fft();
    } else {
        inverse_fft();
    }
}
