layout(location = 0) in vec4 position;
layout(location = 1) in vec4 incident_direction;
layout(location = 2) in vec4 outgoing_direction;
layout(location = 3) in vec4 incident_throughput;
layout(location = 4) in vec4 outgoing_throughput;

flat out uvec4 table_data;

#include <common.glsl>
#include <random.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>

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

#define FILTER_DELTA (globals.filter_delta)
#define FRAME_RANDOM (globals.frame_state.xy)
#define FRAME_NUMBER (globals.frame_state.z)

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

ivec2 hash_position(vec3 pos) {
    uvec3 cell = floatBitsToUint(floor(pos / globals.grid_cell_size));

    // int coords = ((cell_x * 395 + cell_y * 119 + cell_z * 1193) % (4096 * 4096) + 4096 * 4096) % (4096 * 4096);
    // uint coords = (cell_x * 1325290093U + cell_y * 2682811433U + cell_z * 765270841U) % (4096U * 4096U);
    uint coords = shuffle(cell, FRAME_RANDOM) % (HASH_TABLE_COLS * HASH_TABLE_ROWS);

    uint cell_dx = uint(gl_InstanceID) & (globals.hash_cell_cols - 1U); // % globals.hash_cell_cols;
    uint cell_dy = uint(gl_InstanceID) >> globals.hash_cell_col_bits; // / globals.hash_cell_cols;

    uint coord_x = coords % HASH_TABLE_COLS;
    uint coord_y = coords / HASH_TABLE_COLS;

    coord_x &= ~(globals.hash_cell_cols - 1U);
    coord_y &= ~(globals.hash_cell_rows - 1U);

    return ivec2(coord_x + cell_dx, coord_y + cell_dy);
}

vec3 get_relative_pos_in_cell(vec3 pos) {
    float cell_x = fract(pos.x / globals.grid_cell_size);
    float cell_y = fract(pos.y / globals.grid_cell_size);
    float cell_z = fract(pos.z / globals.grid_cell_size);

    return vec3(cell_x, cell_y, cell_z);
}

void main() {
    random_t random = rand_initialize_from_seed(uvec2(gl_VertexID, gl_InstanceID) + FRAME_RANDOM);

    ray_t ray;

    // pick a random ray target in the scene's bounding box
    vec3 bbmin, bbmax;

    get_scene_bbox(bbmin, bbmax);

    // pick a random incident ray direction, importance-sampled
    float unused_pdf;
    vec3 throughput = env_sample_light(ray.dir, unused_pdf, random);
    ray.dir = -ray.dir;

    // find the bounding sphere for the scene
    float radius = max(bbmax.x - bbmin.x, max(bbmax.y - bbmin.y, bbmax.z - bbmin.z)) / 2.0 * sqrt(3.0);

    // adjust PDF
    throughput *= M_PI * radius * radius;

    // generate a random "upwards" vector in the unit disk
    vec2 rng1 = rand_uniform_vec2(random);

    float r = sqrt(rng1.x) * radius;
    float a = rng1.y * M_2PI;

    float px = r * cos(a);
    float py = r * sin(a);

    vec3 base_pos = vec3(px, 0.0, py);
    
    // rotate it to be aligned with the ray direction
    vec3 real_pos = rotate(base_pos, ray.dir);

    // compute a good ray origin
    ray.org = real_pos - 100.0 * ray.dir;

    // now fire the ray at the world, hoping for an intersection
    uint flags;
    int diffuse_bounces = 0;

    for (uint bounce = 0U; bounce < 8U; ++bounce) {
        traversal_t traversal = traverse_scene(ray, 0U);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;

            vec3 last_throughput = throughput;
            vec3 radiance; // unused

            float p = 0.5; // max(last_throughput.x, max(last_throughput.y, last_throughput.z));
            bool pass = false;

            vec2 rng = rand_uniform_vec2(random);

            if (rng.x < p) {
                pass = true;
                last_throughput /= p;
                throughput /= p;
            }

            // TODO: don't hardcode this constant later
            bool is_receiver = (material & 0x8000U) != 0U;
            material &= ~0x8000U;

            if (is_receiver && !pass) {
                // deposit with russian roulette probability
                // if we pass the check, the photon always continues
                // else, it is deposited if possible, or is killed

                

                /*vec2 rng = rand_uniform_vec2(random);

                throughput /= 0.5;
                last_throughput /= 0.5;

                if (rng.x < 0.5) {*/
                    // resolution is 4096 x 4096... assume a grid resolution of 0.5cm for now

                    ivec2 coords = hash_position(ray.org);

                    gl_PointSize = 1.0;
                    gl_Position = vec4(2.0 * (vec2(0.5) + vec2(coords)) / vec2(float(HASH_TABLE_COLS), float(HASH_TABLE_ROWS)) - 1.0, 0.0, 1.0);

                    float sgn = (ray.dir.z < 0.0) ? -1.0 : 1.0;

                    vec3 cell_pos = floor(ray.org / globals.grid_cell_size) * globals.grid_cell_size;
                    vec3 relative_position = ray.org - cell_pos;

                    table_data.r = packHalf2x16(relative_position.xy);
                    table_data.g = packHalf2x16(vec2(relative_position.z, ray.dir.x));
                    table_data.b = packHalf2x16(vec2(ray.dir.y, last_throughput.r));
                    table_data.a = packHalf2x16(vec2(last_throughput.g, last_throughput.b * sgn));

                    /*table_data.r = floatBitsToUint(ray.org.x);
                    table_data.g = floatBitsToUint(ray.org.y);
                    table_data.b = floatBitsToUint(ray.org.z);
                    table_data.a = 1U;*/
                    return;
                } else if (pass) {

            // interact with the material, get the new direction...
            ray_t new_ray = mat_interact(material, mat_inst, normal, -ray.dir, ray.org, traversal.range.y, throughput, radiance, flags, random);

            if ((flags & RAY_FLAG_EXTINCT) != 0U) {
                gl_PointSize = 1.0;
                gl_Position = vec4(-1.0, -1.0, -1.0, 1.0);
                return;
            }

            ray = new_ray;
                } else {
                    break;
                }

            // not a diffuse material, keep bouncing...
            // TODO: russian roulette
        } else {
            // no hit, consider photon lost
            gl_PointSize = 1.0;
            gl_Position = vec4(-1.0, -1.0, -1.0, 1.0);
            return;
        }
    }
}
