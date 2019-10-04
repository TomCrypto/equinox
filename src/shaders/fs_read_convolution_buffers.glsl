// #define CONV_DIMS            vec2      <dimensions of the entire convolution buffer>
// #define IMAGE_DIMS           vec2      <dimensions of the output image to read out>

out vec3 target;

uniform sampler2D r_conv_buffer;
uniform sampler2D g_conv_buffer;
uniform sampler2D b_conv_buffer;

uniform sampler2D source;

// const vec3 WEIGHT = vec3(0.5515, 0.4946, 0.4451); // TODO: depends on the aperture!
// const vec3 WEIGHT = vec3(0.3158, 0.2706, 0.2174);
// const vec3 WEIGHT = vec3(0.1397, 0.1171, 0.0978);
const vec3 WEIGHT = vec3(0.2769, 0.2376, 0.2036);

const float NORMALIZATION = 1.0 / (CONV_DIMS.x * CONV_DIMS.y);

void main() {
    vec2 p = (0.5 - 1.0 / CONV_DIMS) * (gl_FragCoord.xy - 0.5) / (IMAGE_DIMS - 1.0);
    p += 0.5 / CONV_DIMS;

    // Normalize the output data from the FFT -> IFFT step
    float r = texture(r_conv_buffer, p).r * NORMALIZATION;
    float g = texture(g_conv_buffer, p).r * NORMALIZATION;
    float b = texture(b_conv_buffer, p).r * NORMALIZATION;

    target = vec3(r, g, b) + texelFetch(source, ivec2(gl_FragCoord.xy - 0.5), 0).rgb * WEIGHT;
}
