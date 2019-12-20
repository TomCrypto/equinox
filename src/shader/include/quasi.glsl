// requires-define SAMPLER_MAX_DIMENSIONS

layout (std140) uniform QuasiSampler {
    uvec4 alpha[SAMPLER_MAX_DIMENSIONS];
} quasi_buffer;

struct quasi_t {
    uvec2 sw;
    uint dim;
};

uint multiply_high(uint x, uint y) {
    uint xhi = x & 0xFFFFU, xlo = x >> 16U;
    uint yhi = y & 0xFFFFU, ylo = y >> 16U;

    return xhi * yhi + (xhi * ylo) >> 16U + (xlo * yhi) >> 16U;
}

quasi_t quasi_init(uint lo, uint hi) {
    return quasi_t(uvec2(hi, lo), 0U);
}

float quasi_sample(inout quasi_t state) {
    uvec4 param = quasi_buffer.alpha[state.dim++];

    uint product = param.y * state.sw.x
                 + param.x * state.sw.y
                 + multiply_high(param.z, state.sw.x)
                 + multiply_high(param.y, state.sw.y);

    return fract(0.5 + uintBitsToFloat(0x3F800000U | (product >> 9U)));
}
