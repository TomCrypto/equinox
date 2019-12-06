layout(location = 0) out vec4 convolution_output;

uniform sampler2D signal_tile_r;
uniform sampler2D signal_tile_g;
uniform sampler2D signal_tile_b;

void main() {
    float signal_r = texelFetch(signal_tile_r, ivec2(gl_FragCoord.xy - 0.5), 0).r;
    float signal_g = texelFetch(signal_tile_g, ivec2(gl_FragCoord.xy - 0.5), 0).g;
    float signal_b = texelFetch(signal_tile_b, ivec2(gl_FragCoord.xy - 0.5), 0).b;

    // TODO: need a normalization factor depending on the tile size here...
    float normalization = 1.0 / (512.0 * 512.0);

    signal_r *= normalization;
    signal_g *= normalization;
    signal_b *= normalization;

    convolution_output = vec4(signal_r, signal_g, signal_b, 1.0);
}
