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

#define BUTTERFLY   (int(fft_pass_data.z))
#define HORIZONTAL  (fft_pass_data.x == 1U)
#define DIRECTION   (fft_pass_data.y == 1U)
#define CONVOLVE    (fft_pass_data.w == 1U)

// DIF-FFT, ordered -> scrambled
void forward_fft(int i, int j) {
    int ti = (2 * j < BUTTERFLY) ? i : (i - BUTTERFLY / 2);
    int ui = (2 * j < BUTTERFLY) ? (i + BUTTERFLY / 2) : i;

    vec2 rt, gt, bt;
    vec2 ru, gu, bu;

    if (HORIZONTAL) {
        int y = int(gl_FragCoord.y - 0.5);

        rt = texelFetch(r_conv_buffer, ivec2(ti, y), 0).rg;
        ru = texelFetch(r_conv_buffer, ivec2(ui, y), 0).rg;
        gt = texelFetch(g_conv_buffer, ivec2(ti, y), 0).rg;
        gu = texelFetch(g_conv_buffer, ivec2(ui, y), 0).rg;
        bt = texelFetch(b_conv_buffer, ivec2(ti, y), 0).rg;
        bu = texelFetch(b_conv_buffer, ivec2(ui, y), 0).rg;
    } else {
        int x = int(gl_FragCoord.x - 0.5);

        rt = texelFetch(r_conv_buffer, ivec2(x, ti), 0).rg;
        ru = texelFetch(r_conv_buffer, ivec2(x, ui), 0).rg;
        gt = texelFetch(g_conv_buffer, ivec2(x, ti), 0).rg;
        gu = texelFetch(g_conv_buffer, ivec2(x, ui), 0).rg;
        bt = texelFetch(b_conv_buffer, ivec2(x, ti), 0).rg;
        bu = texelFetch(b_conv_buffer, ivec2(x, ui), 0).rg;
    }

    if (2 * j < BUTTERFLY) {
        r_conv_output = rt + ru;
        g_conv_output = gt + gu;
        b_conv_output = bt + bu;
        return; // no twiddling
    }

    ru -= rt;
    gu -= gt;
    bu -= bt;

    float twiddle = M_2PI * float(j) / float(BUTTERFLY);

    float cosW = cos(twiddle);
    float sinW = sin(twiddle);

    r_conv_output = vec2(cosW * ru.x + sinW * ru.y, cosW * ru.y - sinW * ru.x);
    g_conv_output = vec2(cosW * gu.x + sinW * gu.y, cosW * gu.y - sinW * gu.x);
    b_conv_output = vec2(cosW * bu.x + sinW * bu.y, cosW * bu.y - sinW * bu.x);
}

// DIT-FFT, scrambled -> ordered
void inverse_fft(int i, int j) {
    int ti = (j < BUTTERFLY / 2) ? i : (i - BUTTERFLY / 2);
    int ui = (j < BUTTERFLY / 2) ? (i + BUTTERFLY / 2) : i;

    vec2 rt, gt, bt;
    vec2 ru, gu, bu;

    if (HORIZONTAL) {
        int y = int(gl_FragCoord.y - 0.5);

        rt = texelFetch(r_conv_buffer, ivec2(ti, y), 0).rg;
        ru = texelFetch(r_conv_buffer, ivec2(ui, y), 0).rg;
        gt = texelFetch(g_conv_buffer, ivec2(ti, y), 0).rg;
        gu = texelFetch(g_conv_buffer, ivec2(ui, y), 0).rg;
        bt = texelFetch(b_conv_buffer, ivec2(ti, y), 0).rg;
        bu = texelFetch(b_conv_buffer, ivec2(ui, y), 0).rg;
    } else {
        int x = int(gl_FragCoord.x - 0.5);

        rt = texelFetch(r_conv_buffer, ivec2(x, ti), 0).rg;
        ru = texelFetch(r_conv_buffer, ivec2(x, ui), 0).rg;
        gt = texelFetch(g_conv_buffer, ivec2(x, ti), 0).rg;
        gu = texelFetch(g_conv_buffer, ivec2(x, ui), 0).rg;
        bt = texelFetch(b_conv_buffer, ivec2(x, ti), 0).rg;
        bu = texelFetch(b_conv_buffer, ivec2(x, ui), 0).rg;
    }

    float twiddle = M_2PI * float(j) / float(BUTTERFLY);

    float cosW = cos(twiddle);
    float sinW = sin(twiddle);

    r_conv_output = rt + vec2(cosW * ru.r - sinW * ru.g, sinW * ru.r + cosW * ru.g);
    g_conv_output = gt + vec2(cosW * gu.r - sinW * gu.g, sinW * gu.r + cosW * gu.g);
    b_conv_output = bt + vec2(cosW * bu.r - sinW * bu.g, sinW * bu.r + cosW * bu.g);
}

void main(void){
    int i = int((HORIZONTAL ? gl_FragCoord.x : gl_FragCoord.y) - 0.5);
    int j = i & (BUTTERFLY - 1); // butterfly is always a power of two

    if (DIRECTION) {
        forward_fft(i, j);
    } else {
        inverse_fft(i, j);
    }

    if (CONVOLVE) {
        ivec2 p = ivec2(gl_FragCoord.xy - vec2(0.5));

        vec2 r_filter = texelFetch(r_conv_filter, p, 0).rg;
        vec2 g_filter = texelFetch(g_conv_filter, p, 0).rg;
        vec2 b_filter = texelFetch(b_conv_filter, p, 0).rg;

        r_conv_output = vec2(r_conv_output.r * r_filter.r - r_conv_output.g * r_filter.g,
                             r_conv_output.g * r_filter.r + r_conv_output.r * r_filter.g);
        g_conv_output = vec2(g_conv_output.r * g_filter.r - g_conv_output.g * g_filter.g,
                             g_conv_output.g * g_filter.r + g_conv_output.r * g_filter.g);
        b_conv_output = vec2(b_conv_output.r * b_filter.r - b_conv_output.g * b_filter.g,
                             b_conv_output.g * b_filter.r + b_conv_output.r * b_filter.g);
    }
}
