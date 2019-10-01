out vec4 target;

uniform sampler2D r_spectrum;
uniform sampler2D g_spectrum;
uniform sampler2D b_spectrum;

uniform sampler2D add;
uniform sampler2D subtract;

#define WIDTH 1380.0
#define HEIGHT 1008.0

void main() {
    // the problem is it's going to be interpolating with mostly black pixels outside of the convolution
    // region. we need the texture fetching to be exact here, but how?

    vec2 offset = 1.5 * vec2(1.0 / 2048.0, 1.0 / 1024.0);
    vec2 range = vec2(0.5) - vec2(3.0 / 2048.0, 3.0 / 1024.0);

    float tx = (gl_FragCoord.x - 0.5) / (WIDTH - 1.0);
    float ty = (gl_FragCoord.y - 0.5) / (HEIGHT - 1.0);

    vec2 coords = offset + vec2(tx, ty) * range;
    
    float r = texture(r_spectrum, coords).r / (1024.0 * 2048.0);
    float g = texture(g_spectrum, coords).r / (1024.0 * 2048.0);
    float b = texture(b_spectrum, coords).r / (1024.0 * 2048.0);

    // first sample the addition texture as-is
    vec3 add_data = texture(add, (gl_FragCoord.xy - vec2(0.5)) / vec2(WIDTH, HEIGHT)).rgb;

    // and sample the subtraction texture
    vec3 subtract_data = texture(subtract, coords).rgb;

    // and now sample it as-if we were sampling it at the convolution resolution (2048x1024)

    target = vec4(r, g, b, 0.0); // + vec4(add_data - subtract_data, 0.0);
}

/*

xt = 0.5 + 0.5/Wc + (1.0 - 0.5 / Wc - 0.5 - 0.5 / Wc) (fragCoord.x - 0.5) / (Wr - 1)

xt = 0.5 + 0.5/Wc + (0.5 - 1 / Wc) (fragCoord.x - 0.5) / (wr - 1)

*/
