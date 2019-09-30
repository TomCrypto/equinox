out vec4 aperture;

void main() {
    float x = gl_FragCoord.x / 1024.0;
    float y = gl_FragCoord.y / 1024.0;

    float value = 0.0;

    x -= 0.25;
    y -= 0.25;

    if (x * x + y * y < 0.7 * 0.7) {
        value = 1.0 / (2048.0 * 1024.0);
    }

    aperture = vec4(vec3(value), 0.0);
}
