#include <common.glsl>
#include <random.glsl>

#define SPLIT 1

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>

layout(location = 0) out vec4 visible_point_buf1;
layout(location = 1) out vec4 visible_point_buf2;
layout(location = 2) out vec4 visible_point_buf3;

layout (std140) uniform Camera {
    vec4 origin_plane[4];
    vec4 target_plane[4];
    vec4 aperture_settings;
} camera;

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
    float pass_count;
    float photons_for_pass;
    float total_photons;
    float grid_cell_size;
    uint hash_cell_cols;
    uint hash_cell_rows;
    float alpha;
} globals;

#define FILTER_DELTA (globals.filter_delta)
#define FRAME_RANDOM (globals.frame_state.xy)
#define FRAME_NUMBER (globals.frame_state.z)

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

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

    random_t random = rand_initialize_from_seed(uvec2(gl_FragCoord.xy) + FRAME_RANDOM);

    ray_t ray;
    evaluate_primary_ray(random, ray.org, ray.dir);

    vec3 throughput = vec3(1.0);
    uint traversal_start = 0U;
    uint flags;
    float unused_pdf;

    for (uint bounce = 0U; bounce < 8U; ++bounce) {
        traversal_t traversal = traverse_scene(ray, traversal_start);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;

            if (mat_is_diffuse(material)) {
                // we found our diffuse surface, record the hit...

                pack_visible_point(ray.org, ray.dir, normal, throughput, material, mat_inst, visible_point_buf1, visible_point_buf2, visible_point_buf3);

                return;
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

        /*if (bounce <= 2U) {
            continue;
        }

        // russian roulette

        vec2 rng = rand_uniform_vec2(random);
        float p = min(1.0, max(throughput.x, max(throughput.y, throughput.z)));

        if (rng.x < p) {
            throughput /= p;
        } else {
            break;
        }*/
    }

    // if we hit nothing or eventually hit a light with no diffuse bounces, record the
    // throughput normally and set a special flag indicating this
    
    // store the position + packed direction in an RGBA32F texture
    // store the throughput + flags in an RGBA16F texture

    pack_invalid_visible_point(radiance, visible_point_buf1, visible_point_buf2, visible_point_buf3);
}
