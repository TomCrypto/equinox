// #define CONV_DIMS            vec2      <dimensions of the entire convolution buffer>
// #define IMAGE_DIMS           vec2      <dimensions of the output image to read out>

out vec4 target;

uniform sampler2D r_conv_buffer;
uniform sampler2D g_conv_buffer;
uniform sampler2D b_conv_buffer;

uniform sampler2D source;

// const vec3 WEIGHT = vec3(0.5515, 0.4946, 0.4451); // TODO: depends on the aperture!
// const vec3 WEIGHT = vec3(0.3158, 0.2706, 0.2174);
// const vec3 WEIGHT = vec3(0.1397, 0.1171, 0.0978);
// const vec3 WEIGHT = vec3(0.2769, 0.2376, 0.2036);
// const vec3 WEIGHT = vec3(0.3154949, 0.27033883, 0.23277324);
const vec3 WEIGHT = vec3(0.35067707, 0.3021619, 0.26073822);

const float NORMALIZATION = 1.0 / (CONV_DIMS.x * CONV_DIMS.y);

void main() {
    vec2 p = (0.5 - 1.0 / CONV_DIMS) * (gl_FragCoord.xy - 0.5) / (IMAGE_DIMS - 1.0);
    p += 0.5 / CONV_DIMS;

    // Normalize the output data from the FFT -> IFFT step
    float r = textureLod(r_conv_buffer, p, 0.0).r * NORMALIZATION;
    float g = textureLod(g_conv_buffer, p, 0.0).r * NORMALIZATION;
    float b = textureLod(b_conv_buffer, p, 0.0).r * NORMALIZATION;

    vec4 value = texelFetch(source, ivec2(gl_FragCoord.xy - 0.5), 0);

    target = vec4(vec3(r, g, b) + value.rgb / value.a * WEIGHT, 1.0);
}
