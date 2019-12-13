layout(location = 0) in uvec4 fft_pass_data_attribute;

flat out uvec4 fft_pass_data;

void main() {
    gl_Position = vec4(
        float(((gl_VertexID % 3) & 1) << 2) - 1.0,
        float(((gl_VertexID % 3) & 2) << 1) - 1.0,
        0.0,
        1.0
    );

    fft_pass_data = fft_pass_data_attribute;
}
