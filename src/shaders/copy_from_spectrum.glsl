// #define CONV_DIMS            vec2      <dimensions of the entire convolution buffer>
// #define IMAGE_DIMS           vec2      <dimensions of the source image to read out>

out vec3 target;

uniform sampler2D r_spectrum;
uniform sampler2D g_spectrum;
uniform sampler2D b_spectrum;

uniform sampler2D add;
uniform sampler2D subtract;

void main() {
    float tx = (gl_FragCoord.x - 0.5) / (IMAGE_DIMS.x - 1.0);
    float ty = (gl_FragCoord.y - 0.5) / (IMAGE_DIMS.y - 1.0);

    // TODO: when convolution is properly centered, the offset should be 0.5 / ...
    float u = 1.5 / CONV_DIMS.x + tx * (0.5 - 1.0 / CONV_DIMS.x);
    float v = 1.5 / CONV_DIMS.y + ty * (0.5 - 1.0 / CONV_DIMS.y);

    vec2 coords = vec2(u, v);

    float r = texture(r_spectrum, coords).r / (CONV_DIMS.x * CONV_DIMS.y);
    float g = texture(g_spectrum, coords).r / (CONV_DIMS.x * CONV_DIMS.y);
    float b = texture(b_spectrum, coords).r / (CONV_DIMS.x * CONV_DIMS.y);

    target = vec3(r, g, b) + texelFetch(add, ivec2(gl_FragCoord.xy - 0.5), 0).rgb * vec3(0.5515, 0.4946, 0.4451); // * vec3(0.18824, 0.16128, 0.13659);
}


