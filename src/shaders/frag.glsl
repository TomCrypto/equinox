#include <common.glsl>
#include <random.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>

out vec4 result;

layout (std140) uniform Camera {
    vec4 origin_plane[4];
    vec4 target_plane[4];
    vec4 aperture_settings;
} camera;

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
} globals;

#define FILTER_DELTA (globals.filter_delta)
#define FRAME_RANDOM (globals.frame_state.xy)
#define FRAME_NUMBER (globals.frame_state.z)

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

uniform highp usampler2D photon_table;
uniform sampler2D photon_radius_tex;

#define CELL_SIZE 0.04

uvec3 get_cell_for_pos(vec3 pos) {
    uint cell_x = uint(1000.0 + floor(pos.x / CELL_SIZE));
    uint cell_y = uint(1000.0 + floor(pos.y / CELL_SIZE));
    uint cell_z = uint(1000.0 + floor(pos.z / CELL_SIZE));

    return uvec3(cell_x, cell_y, cell_z);
}

ivec2 position_for_cell(uvec3 cell) {
    uint coords = (cell.x * 1325290093U + cell.y * 2682811433U + cell.z * 765270841U) % (4096U * 4096U);
    // uint coords = shuffle(cell) % (4096U * 4096U);

    int coord_x = int(coords % 4096U);
    int coord_y = int(coords / 4096U);

    return ivec2(coord_x, coord_y);
}

vec3 get_photon(uvec3 cell, vec3 point, float radius, uint material, uint inst, vec3 normal, vec3 wo, inout int count) {
    ivec2 coords = position_for_cell(cell);

    uvec4 photon_data = texelFetch(photon_table, coords, 0);

    vec2 data1 = unpackHalf2x16(photon_data.r);
    vec2 data2 = unpackHalf2x16(photon_data.g);
    vec2 data3 = unpackHalf2x16(photon_data.b);
    vec2 data4 = unpackHalf2x16(photon_data.a);

    vec3 photon_position = vec3(data1.xy, data2.x) * 1000.0;
    vec3 photon_throughput = vec3(data3.y, data4.xy);
    
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

int accumulate_photons(uint material, uint inst, vec3 normal, inout vec3 radiance, vec3 throughput, vec3 point, vec3 wo) {
    float radius = min(texelFetch(photon_radius_tex, ivec2(gl_FragCoord.xy - 0.5), 0).w, CELL_SIZE * 2.0);
    int count = 0;

    vec3 accumulation = vec3(0.0);

    // try all surrounding cells, looking for a photon within
    uvec3 center = get_cell_for_pos(point);

    // there's 27 possible points (for now!)
    for (int dx = -1; dx <= 1; ++dx) {
        for (int dy = -1; dy <= 1; ++dy) {
            for (int dz = -1; dz <= 1; ++dz) {
                accumulation += get_photon(uvec3(ivec3(center) + ivec3(dx, dy, dz)), point, radius, material, inst, normal, wo, count);
            }
        }
    }

    radiance += throughput * accumulation / (1e6 * M_PI * radius * radius);

    return count;
}

// Low-discrepancy sequence generator.
//
// Given a fixed, unchanging key, this will produce a low-discrepancy sequence of 2D points
// as a function of frame number, e.g. on the next frame for the same key the next point in
// the sequence will be produced.

vec2 low_discrepancy_2d(uvec2 key) {
    return fract(vec2((key + FRAME_NUMBER) % 8192U) * vec2(0.7548776662, 0.5698402909));
}

// Begin camera stuff

vec2 evaluate_circular_aperture_uv(inout random_t random) {
    vec2 uv = rand_uniform_vec2(random);

    float a = uv.s * M_2PI;

    return sqrt(uv.t) * vec2(cos(a), sin(a));
}

vec2 evaluate_polygon_aperture_uv(inout random_t random) {
    vec2 uv = rand_uniform_vec2(random); // low_discrepancy_2d(pixel_state);

    float corner = floor(uv.s * camera.aperture_settings.y);

    float u = 1.0 - sqrt(uv.s * camera.aperture_settings.y - corner);
    float v = uv.t * (1.0 - u);

    float a = M_PI * camera.aperture_settings.w;

    float rotation = camera.aperture_settings.z + corner * 2.0 * a;

    float c = cos(rotation);
    float s = sin(rotation);

    vec2 p = vec2((u + v) * cos(a), (u - v) * sin(a));
    return vec2(c * p.x - s * p.y, s * p.x + c * p.y);
}

vec2 evaluate_aperture_uv(inout random_t random) {
    switch (int(camera.aperture_settings.x)) {
        case 0: return evaluate_circular_aperture_uv(random);
        case 1: return evaluate_polygon_aperture_uv(random);       
    }

    return vec2(0.0);
}

vec3 bilinear(vec4 p[4], vec2 uv) {
    return mix(mix(p[0].xyz, p[1].xyz, uv.x), mix(p[2].xyz, p[3].xyz, uv.x), uv.y);
}

void evaluate_primary_ray(inout random_t random, out vec3 pos, out vec3 dir) {
    vec2 raster_uv = (gl_FragCoord.xy + FILTER_DELTA) * raster.dimensions.w;
    raster_uv.x -= (raster.dimensions.x * raster.dimensions.w - 1.0) * 0.5;

    vec3 origin = bilinear(camera.origin_plane, evaluate_aperture_uv(random) * 0.5 + 0.5);

    // TODO: this isn't quite right; this generates a flat focal plane but it should be curved
    // (to be equidistant to the lens)
    // maybe just generate this directly in the shader, pass in the camera kind/parameters
    // but it will do for now, we can extend it later when it's needed

    vec3 target = bilinear(camera.target_plane, raster_uv);

    pos = origin;
    dir = normalize(target - origin);
}

// End camera stuff

void main() {
    vec3 radiance = vec3(0.0);
    int count = 0;

    random_t random = rand_initialize_from_seed(uvec2(gl_FragCoord.xy) + FRAME_RANDOM);

    ray_t ray;
    evaluate_primary_ray(random, ray.org, ray.dir);

    vec3 throughput = vec3(1.0);
    uint traversal_start = 0U;
    uint flags;
    float unused_pdf;

    for (uint bounce = 0U; bounce < 100U; ++bounce) {
        traversal_t traversal = traverse_scene(ray, traversal_start);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;

            // if (mat_is_diffuse(material)) {
            if (true) {
                // DIFFUSE SURFACE: query the photon map to accumulate incident photons
                // TODO: this doesn't account properly for absorption up to this point...
                count = accumulate_photons(material, mat_inst, normal, radiance, throughput, ray.org, -ray.dir);
                //radiance += vec3(0.0, 3.0, 0.0);
                break;
            } else {
                // NOT DIFFUSE: just keep tracing as usual...
                ray = mat_interact(material, mat_inst, normal, -ray.dir, ray.org, traversal.range.y, throughput, radiance, flags, random);

                if ((flags & RAY_FLAG_EXTINCT) != 0U) {
                    break; // no need to trace further
                }

                if (((~flags) & (RAY_FLAG_OUTSIDE | RAY_FLAG_TRANSMIT)) == 0U) {
                    traversal_start = traversal.hit.z;
                } else {
                    traversal_start = 0U;
                }
            }            
        } else {
            if ((flags & RAY_FLAG_ENVMAP_SAMPLED) == 0U) {
                radiance += throughput * env_eval_light(ray.dir, unused_pdf);
            }

            break;
        }

        if (bounce <= 2U) {
            continue;
        }

        // russian roulette

        vec2 rng = rand_uniform_vec2(random);
        float p = min(1.0, max(throughput.x, max(throughput.y, throughput.z)));

        if (rng.x < p) {
            throughput /= p;
        } else {
            break;
        }
    }

    result = vec4(radiance, float(count));
}
