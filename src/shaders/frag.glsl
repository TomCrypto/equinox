#include <common.glsl>

#include <material_basic.glsl>

uniform sampler2D visible_point_path_buf1;
uniform sampler2D visible_point_path_buf2;
uniform sampler2D visible_point_path_buf3;

out vec4 result;

uniform highp usampler2D photon_table;
uniform sampler2D photon_radius_tex;

#define CELL_SIZE 0.03

uvec3 get_cell_for_pos(vec3 pos) {
    uint cell_x = uint(100 + int(floor(pos.x / CELL_SIZE)));
    uint cell_y = uint(100 + int(floor(pos.y / CELL_SIZE)));
    uint cell_z = uint(100 + int(floor(pos.z / CELL_SIZE)));

    return uvec3(cell_x, cell_y, cell_z);
}

ivec2 position_for_cell(uvec3 cell) {
    // uint coords = (cell.x * 1325290093U + cell.y * 2682811433U + cell.z * 765270841U) % (4096U * 4096U);
    uint coords = shuffle(cell) % (4096U * 4096U);

    int coord_x = int(coords % 4096U);
    int coord_y = int(coords / 4096U);

    return ivec2(coord_x, coord_y);
}

vec3 get_photon(vec3 cell_pos, uvec3 cell, vec3 point, float radius, uint material, uint inst, vec3 normal, vec3 wo, inout int count) {
    ivec2 coords = position_for_cell(cell);

    uvec4 photon_data = texelFetch(photon_table, coords, 0);

    vec2 data1 = unpackHalf2x16(photon_data.r);
    vec2 data2 = unpackHalf2x16(photon_data.g);
    vec2 data3 = unpackHalf2x16(photon_data.b);
    vec2 data4 = unpackHalf2x16(photon_data.a);

    vec3 photon_position = vec3(data1.xy, data2.x);
    vec3 photon_throughput = vec3(data3.y, data4.xy);

    if (data2.y == 0.0 && data3.x == 0.0) {
        return vec3(0.0); // no photon
    }

    // photon_position = photon_position + cell_pos;

    // vec3 photon_position = vec3(cell) * CELL_SIZE + photon_relative_position;

    float sgn = (photon_throughput.b < 0.0) ? -1.0 : 1.0;

    vec3 photon_direction = vec3(data2.y, data3.x, sqrt(1.0 - data2.y * data2.y - data3.x * data3.x) * sgn);

    photon_throughput.b *= sgn;

    if (distance(point, photon_position) <= radius) {
        float pdf;
        count += 1;
        return max(0.0, dot(-photon_direction, normal)) * photon_throughput * mat_eval_brdf(material, inst, normal, -photon_direction, wo, pdf);
    } else {
        return vec3(0.0);
    }
}

void main() {
    vec4 data1 = texelFetch(visible_point_path_buf1, ivec2(gl_FragCoord.xy - 0.5), 0);
    vec4 data2 = texelFetch(visible_point_path_buf2, ivec2(gl_FragCoord.xy - 0.5), 0);
    vec4 data3 = texelFetch(visible_point_path_buf3, ivec2(gl_FragCoord.xy - 0.5), 0);

    vec3 position, direction, throughput, normal;
    uint material, inst;

    if (!unpack_visible_point(data1, data2, data3, position, direction, normal, throughput, material, inst)) {
        // no visible point, don't do anything
        result = vec4(throughput, 1.0); // TODO: not sure what count to use here?
    } else {
        // at this point, just accumulate all nearby photons
        float radius = texelFetch(photon_radius_tex, ivec2(gl_FragCoord.xy - 0.5), 0).w;
        int count = 0;

        vec3 accumulation = vec3(0.0);

        // try all surrounding cells, looking for a photon within
        uvec3 center = get_cell_for_pos(position);

        // there's 27 possible points (for now!)
        for (int dx = -1; dx <= 1; ++dx) {
            for (int dy = -1; dy <= 1; ++dy) {
                for (int dz = -1; dz <= 1; ++dz) {
                    vec3 cell_pos = floor(position / CELL_SIZE) * CELL_SIZE + vec3(float(dx), float(dy), float(dz)) * CELL_SIZE;

                    accumulation += get_photon(cell_pos, uvec3(ivec3(center) + ivec3(dx, dy, dz)), position, radius, material, inst, normal, -direction, count);
                }
            }
        }

        vec3 radiance = throughput * accumulation / (50000.0 * M_PI * radius * radius);

        result = vec4(radiance, count);
    }
}
