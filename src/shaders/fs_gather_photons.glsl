#include <common.glsl>
#include <random.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>
#include <integrator.glsl>

uniform sampler2D photon_table_pos;
uniform sampler2D photon_table_dir;
uniform sampler2D photon_table_sum;

layout(location = 0) out vec4 radiance_estimate;

layout (std140) uniform Camera {
    vec4 origin_plane[4];
    vec4 target_plane[4];
    vec4 aperture_settings;
} camera;

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

vec3 get_photon(vec3 cell_pos, vec3 point, uint material, uint inst, vec3 normal, vec3 wo) {
    float radius_squared = integrator.search_radius * integrator.search_radius;

    ivec2 coords = hash_entry_for_cell(cell_pos);

    vec3 result = vec3(0.0);

    for (uint y = 0U; y < integrator.hash_cell_rows; ++y) {
        for (uint x = 0U; x < integrator.hash_cell_cols; ++x) {
            vec3 photon_throughput = 1e5 * texelFetch(photon_table_sum, coords + ivec2(x, y), 0).rgb;

            if (photon_throughput == vec3(0.0)) {
                continue;
            }

            vec3 pos_data = texelFetch(photon_table_pos, coords + ivec2(x, y), 0).rgb;
            vec3 photon_wi = 2.0 * texelFetch(photon_table_dir, coords + ivec2(x, y), 0).rgb - 1.0;

            vec3 photon_position = (cell_pos + pos_data) * integrator_cell_size();

            if (dot(point - photon_position, point - photon_position) > radius_squared) {
                continue;
            }

            float kernel_r = distance(point, photon_position) / integrator.search_radius;

            float weight = sin(M_PI * (0.5 + kernel_r * 0.5));
            weight *= weight;

            float pdf;
            result += weight * abs(photon_throughput) * mat_eval_brdf(material, inst, normal, photon_wi, wo, pdf);
        }
    }

    return result;
}

vec3 gather_photons_in_sphere(vec3 position, vec3 wo, vec3 normal, uint material, uint inst) {
    vec3 cell_pos = floor(position / integrator_cell_size());
    vec3 in_pos = fract(position / integrator_cell_size());

    vec3 dir = sign(in_pos - vec3(0.5));

    vec3 accumulation = vec3(0.0);

    accumulation += get_photon(cell_pos + dir * vec3(0.0, 0.0, 0.0), position, material, inst, normal, wo);
    accumulation += get_photon(cell_pos + dir * vec3(0.0, 0.0, 1.0), position, material, inst, normal, wo);
    accumulation += get_photon(cell_pos + dir * vec3(0.0, 1.0, 0.0), position, material, inst, normal, wo);
    accumulation += get_photon(cell_pos + dir * vec3(0.0, 1.0, 1.0), position, material, inst, normal, wo);
    accumulation += get_photon(cell_pos + dir * vec3(1.0, 0.0, 0.0), position, material, inst, normal, wo);
    accumulation += get_photon(cell_pos + dir * vec3(1.0, 0.0, 1.0), position, material, inst, normal, wo);
    accumulation += get_photon(cell_pos + dir * vec3(1.0, 1.0, 0.0), position, material, inst, normal, wo);
    accumulation += get_photon(cell_pos + dir * vec3(1.0, 1.0, 1.0), position, material, inst, normal, wo);

    return accumulation;
}

// Begin camera stuff

vec2 evaluate_circular_aperture_uv(inout random_t random) {
    vec2 uv = rand_uniform_vec2(random);

    float a = uv.s * M_2PI;

    return sqrt(uv.t) * vec2(cos(a), sin(a));
}

vec2 evaluate_polygon_aperture_uv(inout random_t random) {
    vec2 uv = rand_uniform_vec2(random);

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
    vec2 raster_uv = (gl_FragCoord.xy + integrator.filter_offset) * raster.dimensions.w;
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

vec3 gather_photons(ray_t ray, random_t random) {
    float light_pdf, material_pdf;
    vec3 throughput = vec3(1.0);
    vec3 radiance = vec3(0.0);

    bool mis = false;

    for (uint bounce = 0U; bounce < integrator.max_gather_bounces; ++bounce) {
        traversal_t traversal = traverse_scene(ray, 0U);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;

            bool is_receiver = MAT_IS_RECEIVER(material);

            bool inside = dot(ray.dir, normal) > 0.0;

            mis = MAT_SAMPLE_EXPLICIT(material) && (bounce != integrator.max_gather_bounces - 1U)
                                                && !inside;

            vec3 wi, f, mis_f, mis_wi;
            float mis_material_pdf;

            light_pdf = 0.0;
            vec3 light = mis ? env_sample_light(mis_wi, light_pdf, random) : vec3(0.0);

            #define MAT_SWITCH_LOGIC(absorption, eval, sample) {                                  \
                throughput *= absorption(mat_inst, inside, traversal.range.y);                    \
                                                                                                  \
                if (light_pdf != 0.0) {                                                           \
                    mis_f = eval(mat_inst, normal, mis_wi, -ray.dir, mis_material_pdf)            \
                          * abs(dot(mis_wi, normal)) * throughput;                                \
                }                                                                                 \
                                                                                                  \
                f = sample(mat_inst, normal, wi, -ray.dir, material_pdf, random);                 \
            }

            MAT_DO_SWITCH(material)
            #undef MAT_SWITCH_LOGIC

            if (!is_receiver) {
                float q = max(0.0, 1.0 - luminance(throughput * f) / luminance(throughput));

                if (rand_uniform_float(random) < q) {
                    return radiance;
                }

                float adjustment = 1.0 / (1.0 - q);

                throughput *= f * adjustment;
                mis_f *= adjustment;
            }

            if (light_pdf != 0.0 && mis_material_pdf != 0.0) {
                if (!is_ray_occluded(make_ray(ray.org, mis_wi, normal), 1.0 / 0.0)) {
                    radiance += mis_f * light * power_heuristic(light_pdf, mis_material_pdf);
                }
            }

            if (is_receiver) {
                if (mis) {
                    // Finish the MIS direct light sampling procedure we started earlier; this
                    // is done to ensure that the MIS weights result in an unbiased estimator.

                    if (!is_ray_occluded(make_ray(ray.org, wi, normal), 1.0 / 0.0)) {
                        vec3 light = env_eval_light(wi, light_pdf);

                        radiance += throughput * f * light * power_heuristic(material_pdf, light_pdf);
                    }
                }

                vec3 li = throughput * gather_photons_in_sphere(ray.org, -ray.dir, normal, material, mat_inst);

                // this factor comes from the integral of the kernel over the disk of radius R
                radiance += li / (integrator.photons_for_pass * 0.148679 * M_2PI * integrator.search_radius * integrator.search_radius);
                return radiance;
            }

            ray = make_ray(ray.org, wi, normal);
        } else {
            // If we began an MIS direct light sampling procedure in the previous bounce, finish
            // it now; the ray was clearly not occluded so accumulate the light with MIS weight.

            vec3 light = env_eval_light(ray.dir, light_pdf);

            radiance += throughput * light * (mis ? power_heuristic(material_pdf, light_pdf) : 1.0);

            return radiance;
        }
    }

    return radiance;
}

void main() {
    random_t random = rand_initialize_from_seed(uvec2(gl_FragCoord.xy) + integrator.rng);

    ray_t ray;
    evaluate_primary_ray(random, ray.org, ray.dir);

    radiance_estimate = vec4(gather_photons(ray, random), 1.0);
}
