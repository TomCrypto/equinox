#include <common.glsl>

#include <random.glsl>

#define cell_t vec3

layout (std140) uniform Integrator {
    uvec2 rng;
    vec2 filter_offset;

    uint current_pass;
    float photon_rate;
    float photon_count;
    float sppm_alpha;

    float search_radius;
    uint hash_cell_cols;
    uint hash_cell_rows;
    uint hash_cell_col_bits;

    uint hash_cols_mask;
    uint hash_rows_mask;

    vec2 hash_dimensions;

    uint max_scatter_bounces;
    uint max_gather_bounces;

    float photons_for_pass;
} integrator;

float integrator_cell_size() {
    return integrator.search_radius * 2.0;
}

cell_t cell_for_point(vec3 point) {
    return floor(point / integrator_cell_size());
}

uint internal_hash_cell_key(uvec3 key, uvec2 seed) {
    uvec2 state = key.xy ^ seed;

    _rand_mix_mini(state);
    state ^= key.z ^ seed;
    _rand_mix_mini(state);

    return state.x ^ state.y;
}

ivec2 hash_entry_for_cell(cell_t cell) {
    uint index = internal_hash_cell_key(floatBitsToUint(cell), integrator.rng);

    return ivec2(index & integrator.hash_cols_mask, (index >> 16U) & integrator.hash_rows_mask);
}

ivec2 hash_entry_for_cell(cell_t cell, uint index) {
    return hash_entry_for_cell(cell) + ivec2(index & (integrator.hash_cell_cols - 1U),
                                             index >> integrator.hash_cell_col_bits);
}
