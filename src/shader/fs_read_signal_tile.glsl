layout(location = 0) out vec3 convolution_output;

uniform sampler2D signal_tile_r;
uniform sampler2D signal_tile_g;
uniform sampler2D signal_tile_b;

uniform ivec2 tile_offset;

void main() {
    float signal_r = texelFetch(signal_tile_r, ivec2(gl_FragCoord.xy - 0.5) - tile_offset, 0).r;
    float signal_g = texelFetch(signal_tile_g, ivec2(gl_FragCoord.xy - 0.5) - tile_offset, 0).r;
    float signal_b = texelFetch(signal_tile_b, ivec2(gl_FragCoord.xy - 0.5) - tile_offset, 0).r;

    // TODO: need a normalization factor depending on the tile size here...
    float normalization = 1.0 / (512.0 * 512.0);

    signal_r *= normalization;
    signal_g *= normalization;
    signal_b *= normalization;

    convolution_output = vec3(signal_r, signal_g, signal_b);
}
