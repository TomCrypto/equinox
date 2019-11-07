#include <common.glsl>
#include <random.glsl>

#include <material_basic.glsl>

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
    float pass_count;
    float photons_for_pass;
    float total_photons;
    float grid_cell_size;
    uint hash_cell_cols;
    uint hash_cell_rows;
    uint hash_cell_col_bits;
    float alpha;
} globals;

uniform sampler2D visible_point_path_buf1;
uniform sampler2D visible_point_path_buf2;
uniform sampler2D visible_point_path_buf3;

out vec4 result;

uniform highp usampler2D photon_table;
uniform sampler2D photon_radius_tex;

#define FRAME_RANDOM (globals.frame_state.xy)

ivec2 base_coords(vec3 cell) {
    uvec3 cell_hash_seed = floatBitsToUint(cell);

    // uint coords = (cell.x * 1325290093U + cell.y * 2682811433U + cell.z * 765270841U) % (4096U * 4096U);
    uint coords = shuffle(cell_hash_seed, FRAME_RANDOM) % (HASH_TABLE_COLS * HASH_TABLE_ROWS);

    uint coord_x = coords % HASH_TABLE_COLS;
    uint coord_y = coords / HASH_TABLE_COLS;

    coord_x &= ~(globals.hash_cell_cols - 1U);
    coord_y &= ~(globals.hash_cell_rows - 1U);

    return ivec2(coord_x, coord_y);
}

vec3 get_photon(vec3 cell_pos, vec3 point, float radius_squared, uint material, uint inst, vec3 normal, vec3 wo, inout int count) {
    ivec2 coords = base_coords(cell_pos);

    vec3 result = vec3(0.0);

    for (uint y = 0U; y < globals.hash_cell_rows; ++y) {
        for (uint x = 0U; x < globals.hash_cell_cols; ++x) {
            uvec4 photon_data = texelFetch(photon_table, coords + ivec2(x, y), 0);

            vec2 data1 = unpackHalf2x16(photon_data.r);
            vec2 data2 = unpackHalf2x16(photon_data.g);
            vec2 data3 = unpackHalf2x16(photon_data.b);
            vec2 data4 = unpackHalf2x16(photon_data.a);

            vec3 photon_position = cell_pos * globals.grid_cell_size + vec3(data1.xy, data2.x);
            vec3 photon_throughput = vec3(data3.y, data4.xy);

            if (data2.y == 0.0 && data3.x == 0.0) {
                continue;
            }

            float sgn = (photon_throughput.b < 0.0) ? -1.0 : 1.0;

            vec3 photon_direction = vec3(data2.y, data3.x, sqrt(max(0.0, 1.0 - data2.y * data2.y - data3.x * data3.x)) * sgn);

            photon_throughput.b *= sgn;

            if (dot(point - photon_position, point - photon_position) <= radius_squared) {
                float pdf;
                count += 1;
                result += max(0.0, dot(-photon_direction, normal)) * photon_throughput * mat_eval_brdf(material, inst, normal, -photon_direction, wo, pdf);
            } else {
                continue;
            }
        }
    }

    return result;
}

void main() {
    vec4 data1 = texelFetch(visible_point_path_buf1, ivec2(gl_FragCoord.xy - 0.5), 0);
    vec4 data2 = texelFetch(visible_point_path_buf2, ivec2(gl_FragCoord.xy - 0.5), 0);
    vec4 data3 = texelFetch(visible_point_path_buf3, ivec2(gl_FragCoord.xy - 0.5), 0);

    vec3 position, direction, throughput, normal;
    uint material, inst;

    if (!unpack_visible_point(data1, data2, data3, position, direction, normal, throughput, material, inst)) {
        result = vec4(throughput, 0.0); // count = 0 indicates this is direct lighting
    } else {
        // at this point, just accumulate all nearby photons
        float radius = texelFetch(photon_radius_tex, ivec2(gl_FragCoord.xy - 0.5), 0).w;
        radius = min(radius, globals.grid_cell_size * 2.0);
        float radius_squared = pow(radius, 2.0);
        int count = 0;

        if (radius_squared == 0.0) {
            result = vec4(0.0);
            return;
        }

        vec3 cell_pos = floor(position / globals.grid_cell_size);
        vec3 in_pos = fract(position / globals.grid_cell_size);

        vec3 dir = sign(in_pos - vec3(0.5));

        vec3 accumulation = vec3(0.0);

        accumulation += get_photon(cell_pos + dir * vec3(0.0, 0.0, 0.0), position, radius_squared, material, inst, normal, -direction, count);
        accumulation += get_photon(cell_pos + dir * vec3(0.0, 0.0, 1.0), position, radius_squared, material, inst, normal, -direction, count);
        accumulation += get_photon(cell_pos + dir * vec3(0.0, 1.0, 0.0), position, radius_squared, material, inst, normal, -direction, count);
        accumulation += get_photon(cell_pos + dir * vec3(0.0, 1.0, 1.0), position, radius_squared, material, inst, normal, -direction, count);
        accumulation += get_photon(cell_pos + dir * vec3(1.0, 0.0, 0.0), position, radius_squared, material, inst, normal, -direction, count);
        accumulation += get_photon(cell_pos + dir * vec3(1.0, 0.0, 1.0), position, radius_squared, material, inst, normal, -direction, count);
        accumulation += get_photon(cell_pos + dir * vec3(1.0, 1.0, 0.0), position, radius_squared, material, inst, normal, -direction, count);
        accumulation += get_photon(cell_pos + dir * vec3(1.0, 1.0, 1.0), position, radius_squared, material, inst, normal, -direction, count);

        vec3 radiance = throughput * accumulation;

        result = vec4(radiance, count);
    }
}
