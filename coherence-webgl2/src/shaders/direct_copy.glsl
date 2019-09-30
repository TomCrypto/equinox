out vec4 target;

uniform sampler2D source;

void main() {
    target = vec4(texture(source, (gl_FragCoord.xy - vec2(0.5)) / vec2(1024.0, 512.0)).rgb, 0.0);
}
