#define random_t uvec2

void _rand_mix_full(inout random_t random);
void _rand_mix_mini(inout random_t random);

uint shuffle(uvec3 data, uvec2 seed) {
    uvec2 state = data.xy ^ seed;

    _rand_mix_full(state);

    state.x ^= data.z;
    state.y ^= data.z;

    _rand_mix_full(state);

    return state.x;
}

// Initializes the random generator with a seed.
random_t rand_initialize_from_seed(uvec2 seed) {
    _rand_mix_full(seed);

    return seed;
}

// Generates a pair of uniform floats in [0, 1).
vec2 rand_uniform_vec2(inout random_t random) {
    _rand_mix_mini(random);

    return uintBitsToFloat(0x3f800000U | (random >> 9U)) - 1.0;
}

// Generates a single uniform float in [0, 1).
float rand_uniform_float(inout random_t random) {
    return rand_uniform_vec2(random).x;
}

// Random number generation internals.
//
// This is an implementation of the Speck cipher with a fixed key. The "full" function is used
// when the input is seriously correlated e.g. fragment coordinates, while the "mini" function
// can be called repeatedly on an internal state to produce more random numbers at lower cost.

#define ROR8(x) ((x >> 8U) | (x << 24U))
#define ROL3(x) ((x << 3U) | (x >> 29U))

void _rand_mix_full(inout random_t random) {
    random = uvec2((ROR8(random.x) + random.y) ^ 0x7db90549U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xb3485e6eU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x18f2e32dU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xf9c67acbU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x9c7afc87U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xdc6ea9d2U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x57b5bc6fU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xf85896c0U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x30c9e1b2U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x6d40dabeU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x49343defU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x30bea2b6U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xc914c024U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xdd2bde5bU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x9b2632e3U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xc18a57d0U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xf2ae6e19U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x2bb45906U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x96119df6U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xef5c8ab9U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xcdcc6bd2U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xb54f62b5U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x797d9f4bU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x56b431caU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x8826b611U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x58f1be48U, ROL3(random.y) ^ random.x);
}

void _rand_mix_mini(inout uvec2 random) {
    random = uvec2((ROR8(random.x) + random.y) ^ 0x7db90549U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xb3485e6eU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x18f2e32dU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xf9c67acbU, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0x9c7afc87U, ROL3(random.y) ^ random.x);
    random = uvec2((ROR8(random.x) + random.y) ^ 0xdc6ea9d2U, ROL3(random.y) ^ random.x);
    // add more if needed (or less - need to run some tests probably)
}

#undef ROR8
#undef ROL3
