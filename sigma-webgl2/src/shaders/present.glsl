#version 300 es

precision highp float;

uniform sampler2D samples;

in vec2 clip;

out vec4 color;

void main() {
    // vec4 value = texelFetch(samples, ivec2(gl_FragCoord.xy - 0.5), 0);
    vec4 value = texture(samples, clip);

    color = vec4(value.xyz / value.w, 1.0);
}
