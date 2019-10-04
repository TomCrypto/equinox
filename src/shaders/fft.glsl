#include <common.glsl>

uniform sampler2D r_conv_buffer;
uniform sampler2D g_conv_buffer;
uniform sampler2D b_conv_buffer;

uniform sampler2D r_conv_filter;
uniform sampler2D g_conv_filter;
uniform sampler2D b_conv_filter;

layout(location = 0) out vec2 r_conv_output;
layout(location = 1) out vec2 g_conv_output;
layout(location = 2) out vec2 b_conv_output;

flat in uvec4 fft_pass_data;

#define SUBTRANSFORM_SIZE (int(fft_pass_data.z))
#define HORIZONTAL (fft_pass_data.x == 1U)
#define DIRECTION (fft_pass_data.y == 1U)
#define CONVOLVE (fft_pass_data.w == 1U)

void forward_fft() {
    int i = int((HORIZONTAL ? gl_FragCoord.x : gl_FragCoord.y) - 0.5);
    
    int j = i & (SUBTRANSFORM_SIZE - 1);

    if (2 * j < SUBTRANSFORM_SIZE) {
        if (HORIZONTAL) {
            int z = int(gl_FragCoord.y - 0.5);

            vec2 r_t = texelFetch(r_conv_buffer, ivec2(i, z), 0).rg;
            vec2 g_t = texelFetch(g_conv_buffer, ivec2(i, z), 0).rg;
            vec2 b_t = texelFetch(b_conv_buffer, ivec2(i, z), 0).rg;

            vec2 r_u = texelFetch(r_conv_buffer, ivec2(i + SUBTRANSFORM_SIZE / 2, z), 0).rg;
            vec2 g_u = texelFetch(g_conv_buffer, ivec2(i + SUBTRANSFORM_SIZE / 2, z), 0).rg;
            vec2 b_u = texelFetch(b_conv_buffer, ivec2(i + SUBTRANSFORM_SIZE / 2, z), 0).rg;

            r_conv_output = r_t + r_u;
            g_conv_output = g_t + g_u;
            b_conv_output = b_t + b_u;
        } else {
            int z = int(gl_FragCoord.x - 0.5);

            vec2 r_t = texelFetch(r_conv_buffer, ivec2(z, i), 0).rg;
            vec2 g_t = texelFetch(g_conv_buffer, ivec2(z, i), 0).rg;
            vec2 b_t = texelFetch(b_conv_buffer, ivec2(z, i), 0).rg;

            vec2 r_u = texelFetch(r_conv_buffer, ivec2(z, i + SUBTRANSFORM_SIZE / 2), 0).rg;
            vec2 g_u = texelFetch(g_conv_buffer, ivec2(z, i + SUBTRANSFORM_SIZE / 2), 0).rg;
            vec2 b_u = texelFetch(b_conv_buffer, ivec2(z, i + SUBTRANSFORM_SIZE / 2), 0).rg;

            r_conv_output = r_t + r_u;
            g_conv_output = g_t + g_u;
            b_conv_output = b_t + b_u;
        }
    } else {
        float twiddle = M_2PI * float(j) / float(SUBTRANSFORM_SIZE);

        vec2 w = vec2(-cos(twiddle), sin(twiddle));

        if (HORIZONTAL) {
            int z = int(gl_FragCoord.y - 0.5);

            vec2 r_t = texelFetch(r_conv_buffer, ivec2(i - SUBTRANSFORM_SIZE / 2, z), 0).rg;
            vec2 g_t = texelFetch(g_conv_buffer, ivec2(i - SUBTRANSFORM_SIZE / 2, z), 0).rg;
            vec2 b_t = texelFetch(b_conv_buffer, ivec2(i - SUBTRANSFORM_SIZE / 2, z), 0).rg;

            vec2 r_u = texelFetch(r_conv_buffer, ivec2(i, z), 0).rg;
            vec2 g_u = texelFetch(g_conv_buffer, ivec2(i, z), 0).rg;
            vec2 b_u = texelFetch(b_conv_buffer, ivec2(i, z), 0).rg;

            r_conv_output = complex_mul(w, r_t - r_u);
            g_conv_output = complex_mul(w, g_t - g_u);
            b_conv_output = complex_mul(w, b_t - b_u);
        } else {
            int z = int(gl_FragCoord.x - 0.5);

            vec2 r_t = texelFetch(r_conv_buffer, ivec2(z, i - SUBTRANSFORM_SIZE / 2), 0).rg;
            vec2 g_t = texelFetch(g_conv_buffer, ivec2(z, i - SUBTRANSFORM_SIZE / 2), 0).rg;
            vec2 b_t = texelFetch(b_conv_buffer, ivec2(z, i - SUBTRANSFORM_SIZE / 2), 0).rg;

            vec2 r_u = texelFetch(r_conv_buffer, ivec2(z, i), 0).rg;
            vec2 g_u = texelFetch(g_conv_buffer, ivec2(z, i), 0).rg;
            vec2 b_u = texelFetch(b_conv_buffer, ivec2(z, i), 0).rg;

            r_conv_output = complex_mul(w, r_t - r_u);
            g_conv_output = complex_mul(w, g_t - g_u);
            b_conv_output = complex_mul(w, b_t - b_u);
        }
    }
}

void inverse_fft() {
    int i = int((HORIZONTAL ? gl_FragCoord.x : gl_FragCoord.y) - 0.5);

    int j = i & (SUBTRANSFORM_SIZE - 1);

    float twiddle = -M_2PI * float(j) / float(SUBTRANSFORM_SIZE);

    vec2 w = vec2(cos(twiddle), -sin(twiddle));

    int ti = i - (j / (SUBTRANSFORM_SIZE / 2)) * (SUBTRANSFORM_SIZE / 2);
    int ui = ti + SUBTRANSFORM_SIZE / 2;

    if (HORIZONTAL) {
        int z = int(gl_FragCoord.y - 0.5);

        vec2 r_t = texelFetch(r_conv_buffer, ivec2(ti, z), 0).rg;
        vec2 g_t = texelFetch(g_conv_buffer, ivec2(ti, z), 0).rg;
        vec2 b_t = texelFetch(b_conv_buffer, ivec2(ti, z), 0).rg;

        vec2 r_u = texelFetch(r_conv_buffer, ivec2(ui, z), 0).rg;
        vec2 g_u = texelFetch(g_conv_buffer, ivec2(ui, z), 0).rg;
        vec2 b_u = texelFetch(b_conv_buffer, ivec2(ui, z), 0).rg;

        r_conv_output = r_t + complex_mul(w, r_u);
        g_conv_output = g_t + complex_mul(w, g_u);
        b_conv_output = b_t + complex_mul(w, b_u);
    } else {
        int z = int(gl_FragCoord.x - 0.5);

        vec2 r_t = texelFetch(r_conv_buffer, ivec2(z, ti), 0).rg;
        vec2 g_t = texelFetch(g_conv_buffer, ivec2(z, ti), 0).rg;
        vec2 b_t = texelFetch(b_conv_buffer, ivec2(z, ti), 0).rg;

        vec2 r_u = texelFetch(r_conv_buffer, ivec2(z, ui), 0).rg;
        vec2 g_u = texelFetch(g_conv_buffer, ivec2(z, ui), 0).rg;
        vec2 b_u = texelFetch(b_conv_buffer, ivec2(z, ui), 0).rg;

        r_conv_output = r_t + complex_mul(w, r_u);
        g_conv_output = g_t + complex_mul(w, g_u);
        b_conv_output = b_t + complex_mul(w, b_u);
    }
}

void main(void){
    if (DIRECTION) {
        forward_fft();
    } else {
        inverse_fft();
    }

    if (CONVOLVE) {
        ivec2 coords = ivec2(gl_FragCoord.xy - vec2(0.5));

        vec2 r_aperture = texelFetch(r_conv_filter, coords, 0).rg;
        vec2 g_aperture = texelFetch(g_conv_filter, coords, 0).rg;
        vec2 b_aperture = texelFetch(b_conv_filter, coords, 0).rg;

        r_conv_output = complex_mul(r_conv_output, r_aperture);
        g_conv_output = complex_mul(g_conv_output, g_aperture);
        b_conv_output = complex_mul(b_conv_output, b_aperture);
    }
}
