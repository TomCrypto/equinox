// requires-define CONV_DIMS            <dimensions of the entire convolution buffer>
// requires-define IMAGE_DIMS           <dimensions of the source image to load in>

layout(location = 0) out vec2 r_conv_buffer;
layout(location = 1) out vec2 g_conv_buffer;
layout(location = 2) out vec2 b_conv_buffer;

uniform sampler2D image;

void main() {
    if (gl_FragCoord.x < CONV_DIMS.x / 2.0 && gl_FragCoord.y < CONV_DIMS.y / 2.0) {
        vec2 p = (1.0 - 1.0 / IMAGE_DIMS) * (gl_FragCoord.xy - 0.5) / (CONV_DIMS / 2.0 - 1.0);
        p += 0.5 / IMAGE_DIMS;

        vec4 value = textureLod(image, p, 0.0);

        vec3 texel = value.rgb / value.a;
        r_conv_buffer = vec2(texel.r, 0.0);
        g_conv_buffer = vec2(texel.g, 0.0);
        b_conv_buffer = vec2(texel.b, 0.0);
    } else {
        r_conv_buffer = vec2(0.0);
        g_conv_buffer = vec2(0.0);
        b_conv_buffer = vec2(0.0);
    }
}
