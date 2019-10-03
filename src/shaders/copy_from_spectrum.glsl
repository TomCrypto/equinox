// #define CONV_DIMS            vec2      <dimensions of the entire convolution buffer>
// #define IMAGE_DIMS           vec2      <dimensions of the source image to read out>

out vec4 target;

uniform sampler2D r_spectrum;
uniform sampler2D g_spectrum;
uniform sampler2D b_spectrum;

uniform sampler2D add;
uniform sampler2D subtract;

void main() {
    float tx = (gl_FragCoord.x - 0.5) / (IMAGE_DIMS.x - 1.0);
    float ty = (gl_FragCoord.y - 0.5) / (IMAGE_DIMS.y - 1.0);

    // TODO: when convolution is properly centered, the offset should be 0.5 / ...
    float u = 1.0 / CONV_DIMS.x + tx * (0.5 - 1.0 / CONV_DIMS.x);
    float v = 1.0 / CONV_DIMS.y + ty * (0.5 - 1.0 / CONV_DIMS.y);

    vec2 coords = vec2(u, v);

    float r = texture(r_spectrum, coords).r / (1024.0 * 2048.0);
    float g = texture(g_spectrum, coords).r / (1024.0 * 2048.0);
    float b = texture(b_spectrum, coords).r / (1024.0 * 2048.0);

    // TODO: add in the original data as well; do this once we have updated the aperture kernel
    // to automatically subtract the original data from the convolution

    target = vec4(r, g, b, 0.0);
}
