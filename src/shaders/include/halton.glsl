struct weyl_t {
    uvec4 state;
    uint dim;
};

// TODO: apply UBO size optimization later?

layout (std140) uniform Weyl {
    uvec4 alpha[64];
} weyl;

weyl_t weyl_init(uint lo, uint hi) {
    uint lolo = lo & 0xffffU;
    uint lohi = lo >> 16U;

    return weyl_t(uvec4(lo, hi, lolo, lohi), 0U);
}

float internal_weyl_computation(uvec4 s, uvec4 a) {
    /*uint carry = (a.lohi * s.lolo) >> 16U + (a.lolo * s.lohi) >> 16U + a.lohi * s.lohi;
    
    uint value = carry + a.lo * s.hi + a.hi * s.lo;*/

    uint carry = ((a.w * s.z) >> 16U) + ((a.z * s.w) >> 16U) + a.w * s.w;
    
    uint value = carry + a.x * s.y + a.y * s.x;

    return fract(0.5 + float(value) / 4294967296.0);
}

float weyl_sample_direct(uint dimension, uvec4 state) {
    return internal_weyl_computation(state, weyl.alpha[dimension & 63U]);
}

float weyl_sample(inout weyl_t state) {
    return weyl_sample_direct(state.dim++, state.state);
}

vec2 weyl_sample_vec2(inout weyl_t state) {
    float u1 = weyl_sample(state);
    float u2 = weyl_sample(state);

    return vec2(u1, u2);
}
