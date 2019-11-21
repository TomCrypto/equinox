struct weyl_t {
    uvec4 state;
    uint dim;
};

// TODO: apply UBO size optimization later?

layout (std140) uniform Weyl {
    uvec4 alpha[SAMPLER_MAX_DIMENSIONS];
} weyl;

weyl_t weyl_init(uint lo, uint hi) {
    return weyl_t(uvec4(lo, hi, lo & 0xffffU, lo >> 16U), 0U);
}

float weyl_sample(inout weyl_t state) {
    uvec4 alpha = weyl.alpha[state.dim++];

    uint product = (alpha.z * state.state.w >> 16U)
                 + (alpha.w * state.state.z >> 16U)
                 +  alpha.x * state.state.y
                 +  alpha.y * state.state.x
                 +  alpha.w * state.state.w;

    return fract(0.5 + float(product) * (1.0 / 4294967296.0));
}

vec2 weyl_sample_vec2(inout weyl_t state) {
    float u1 = weyl_sample(state);
    float u2 = weyl_sample(state);

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
