// #define CONV_DIMS            vec2      <dimensions of the entire convolution buffer>
// #define IMAGE_DIMS           vec2      <dimensions of the source image to load in>

layout(location = 0) out vec2 r_conv_buffer;
layout(location = 1) out vec2 g_conv_buffer;
layout(location = 2) out vec2 b_conv_buffer;

uniform sampler2D image;

void main() {
    // Only load the image into the bottom-left quadrant of the convolution buffers
    if (gl_FragCoord.x < CONV_DIMS.x / 2.0 && gl_FragCoord.y < CONV_DIMS.y / 2.0) {
        vec2 interpolation = (gl_FragCoord.xy - 0.5) / (CONV_DIMS / 2.0 - 1.0);
        vec2 p = 1.0 / IMAGE_DIMS * 0.5 + (1.0 - 1.0 / IMAGE_DIMS) * interpolation;

        vec3 texel = texture(image, p).rgb;
        r_conv_buffer = vec2(texel.r, 0.0);
        g_conv_buffer = vec2(texel.g, 0.0);
        b_conv_buffer = vec2(texel.b, 0.0);
    } else {
        r_conv_buffer = vec2(0.0);
        g_conv_buffer = vec2(0.0);
        b_conv_buffer = vec2(0.0);
    }
}
