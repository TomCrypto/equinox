#include <common.glsl>

#include <random.glsl>

#define cell_t vec3

layout (std140) uniform Integrator {
    uvec2 rng;                  // Random bits for the current pass
    uint pass;                  // The pass number, starting at 1

    vec2 filter_offset;         // The camera filter offset

    float photon_count;         // The total number of photons fired
    float sppm_alpha;           // The integrator's alpha parameter

    float cell_size;            // The size in world units of a hash cell
    uint hash_cell_cols;        // The number of columns in a hash cell
    uint hash_cell_rows;        // The number of rows in a hash cell
    uint hash_cell_col_bits;    // The number of bits in hash_cell_cols

    uint hash_cols_mask;
    uint hash_rows_mask;

    vec2 hash_dimensions;
} integrator;

cell_t cell_for_point(vec3 point) {
    return floor(point / integrator.cell_size);
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

/*bool sphere_in_cell(float radius_squared, vec3 center, cell_t cell) {
    // TODO: implement...
}*/

/*bool sphere_cell_test(float radius_squared, vec3 origin) {
    float distance = 0.0;

    distance += sqr(max(origin.x, 0.0)) + sqr(min(origin.x + globals.grid_cell_size, 0.0));
    distance += sqr(max(origin.y, 0.0)) + sqr(min(origin.y + globals.grid_cell_size, 0.0));
    distance += sqr(max(origin.z, 0.0)) + sqr(min(origin.z + globals.grid_cell_size, 0.0));

    return distance <= radius_squared;
}*/
