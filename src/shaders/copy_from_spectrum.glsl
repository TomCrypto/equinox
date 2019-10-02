// #define CONV_DIMS            vec2      <dimensions of the entire convolution buffer>
// #define IMAGE_DIMS           vec2      <dimensions of the source image to load in>

out vec4 target;

uniform sampler2D r_spectrum;
uniform sampler2D g_spectrum;
uniform sampler2D b_spectrum;

uniform sampler2D add;
uniform sampler2D subtract;

void main() {
    /*

    our linear coordinates across the draw call are:

    */

    float tx = (gl_FragCoord.x - 0.5) / (IMAGE_DIMS.x - 1.0);
    float ty = (gl_FragCoord.y - 0.5) / (IMAGE_DIMS.y - 1.0);

    /*

    the min texel we need to hit is (0.5, 0.5) / (CONV_WIDTH, CONV_HEIGHT)

    and the max is given as 0.5 - 0.5 / (CONV_WIDTH, CONV_HEIGHT)

    */

    float u = 0.5 / CONV_DIMS.x + tx * (0.5 - 1.0 / CONV_DIMS.x);
    float v = 0.5 / CONV_DIMS.y + ty * (0.5 - 1.0 / CONV_DIMS.y);

    vec2 coords = vec2(u, v); // + vec2(0.5); // offset for now since we convolve to the top-right quadrant

    float r = texture(r_spectrum, coords).r / (1024.0 * 2048.0);
    float g = texture(g_spectrum, coords).r / (1024.0 * 2048.0);
    float b = texture(b_spectrum, coords).r / (1024.0 * 2048.0);

    // TODO: add in the original data as well; do this once we have updated the aperture kernel
    // to automatically subtract the original data from the convolution

    target = vec4(r, g, b, 0.0);
}
