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

float quasi_sample_float(inout quasi_t state) {
    uvec4 alpha = quasi_buffer.alpha[state.dim++];

    uint product = (alpha.z * state.state.w >> 16U)
                 + (alpha.w * state.state.z >> 16U)
                 +  alpha.x * state.state.y
                 +  alpha.y * state.state.x
                 +  alpha.w * state.state.w;

    return fract(0.5 + float(product) * (1.0 / 4294967296.0));
}

vec2 quasi_sample_vec2(inout quasi_t state) {
    float u1 = quasi_sample_float(state);
    float u2 = quasi_sample_float(state);

    return vec2(u1, u2);
}

// Feeds an input value through a keyed pseudorandom permutation, to decorrelate
// a correlated sequence; this can suppress visual artifacts when used properly.
uint sampler_decorrelate(uint x, uint key) {
    x ^= key;
    x ^= x >> 17U;
    x ^= x >> 10U;
    x *= 0xb36534e5U;
    x ^= x >> 12U;
    x ^= x >> 21U;
    x *= 0x93fc4795U;
    x ^= 0xdf6e307fU;
    x ^= x >> 17U;
    x *= 1U | key >> 18U;

    return x;
}

uint sampler_decorrelate(uint x) {
    // pass in a default key when not given one
    return sampler_decorrelate(x, 0xa8f4c2c1U);
}
