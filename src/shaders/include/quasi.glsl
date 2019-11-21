struct quasi_t {
    uvec4 state;
    uint dim;
};

layout (std140) uniform QuasiSampler {
    uvec4 alpha[SAMPLER_MAX_DIMENSIONS];
} quasi_buffer;

quasi_t quasi_init(uint lo, uint hi) {
    return quasi_t(uvec4(lo, hi, lo & 0xffffU, lo >> 16U), 0U);
}

float quasi_sample(inout quasi_t state) {
    uvec4 alpha = quasi_buffer.alpha[state.dim++];

    uint product = (alpha.z * state.state.w >> 16U)
                 + (alpha.w * state.state.z >> 16U)
                 +  alpha.x * state.state.y
                 +  alpha.y * state.state.x
                 +  alpha.w * state.state.w;

    return fract(0.5 + float(product) * (1.0 / 4294967296.0));
}
