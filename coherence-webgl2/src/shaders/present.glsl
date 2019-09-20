precision highp float;

uniform sampler2D samples;

out vec4 color;

void main() {
    vec4 value = texelFetch(samples, ivec2(gl_FragCoord.xy - 0.5), 0);

    color = vec4(value.xyz, 1.0);
}
