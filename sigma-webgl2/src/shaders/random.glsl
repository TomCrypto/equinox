// Random number generation logic.
//
// This is an implementation of the Speck cipher with a fixed key. The "full" function is used
// when the input is seriously correlated e.g. fragment coordinates, while the "mini" function
// can be called repeatedly on a given state to produce more random numbers at a lower cost.

#define ROR8(x) ((x >> 8U) | (x << 24U))
#define ROL3(x) ((x << 3U) | (x >> 29U))

void bitshuffle_full(inout uvec2 state) {
    state = uvec2((ROR8(state.x) + state.y) ^ 0x7db90549U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xb3485e6eU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x18f2e32dU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xf9c67acbU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x9c7afc87U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xdc6ea9d2U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x57b5bc6fU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xf85896c0U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x30c9e1b2U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x6d40dabeU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x49343defU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x30bea2b6U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xc914c024U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xdd2bde5bU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x9b2632e3U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xc18a57d0U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xf2ae6e19U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x2bb45906U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x96119df6U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xef5c8ab9U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xcdcc6bd2U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xb54f62b5U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x797d9f4bU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x56b431caU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x8826b611U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x58f1be48U, ROL3(state.y) ^ state.x);
}

void bitshuffle_mini(inout uvec2 state) {
    state = uvec2((ROR8(state.x) + state.y) ^ 0x7db90549U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xb3485e6eU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x18f2e32dU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xf9c67acbU, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0x9c7afc87U, ROL3(state.y) ^ state.x);
    state = uvec2((ROR8(state.x) + state.y) ^ 0xdc6ea9d2U, ROL3(state.y) ^ state.x);
    // TODO: add more if needed (or less)
}

#undef ROR8
#undef ROL3

vec2 gen_vec2_uniform(uvec2 state) {
    return vec2(state) / 4294967295.0;
}
