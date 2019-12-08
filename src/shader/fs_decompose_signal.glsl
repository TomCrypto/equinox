layout(location = 0) out vec3 signal;

uniform sampler2D radiance_estimate;

void main() {
    vec4 value = texelFetch(radiance_estimate, ivec2(gl_FragCoord.xy - 0.5), 0);

    signal = value.rgb / value.w;
}
