in vec2 screen_coords;

// TODO: vec2?
layout(location = 0) out vec4 r_spectrum;
layout(location = 1) out vec4 g_spectrum;
layout(location = 2) out vec4 b_spectrum;

#define WIDTH 1380.0
#define HEIGHT 1008.0

uniform sampler2D source;

void main() {
    float tx = (gl_FragCoord.x) / (1024.0);
    float ty = (gl_FragCoord.y) / (512.0);

    vec3 data = texture(source, vec2(tx, ty)).rgb;

    r_spectrum = vec4(vec2(data.r, 0.0), 0.0, 0.0);
    g_spectrum = vec4(vec2(data.g, 0.0), 0.0, 0.0);
    b_spectrum = vec4(vec2(data.b, 0.0), 0.0, 0.0);
}
