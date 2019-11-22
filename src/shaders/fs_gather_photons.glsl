#include <common.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>
#include <integrator.glsl>
#include <camera.glsl>
#include <quasi.glsl>

uniform sampler2D photon_table_pos;
uniform sampler2D photon_table_dir;
uniform sampler2D photon_table_sum;

layout(location = 0) out vec4 radiance_estimate;

vec3 get_photon(cell_t cell, vec3 point, uint material, uint inst, vec3 normal, vec3 wo) {
    ivec2 coords = hash_entry_for_cell(cell);

    vec3 throughput = 1e5 * texelFetch(photon_table_sum, coords, 0).rgb;

    if (throughput == vec3(0.0)) {
        return vec3(0.0);
    }

    vec3 position = (cell + texelFetch(photon_table_pos, coords, 0).rgb) * integrator.cell_size;

    if (dot(point - position, point - position) <= integrator.search_radius_squared) {
        vec3 wi = 2.0 * texelFetch(photon_table_dir, coords, 0).rgb - 1.0;

        #define MAT_SWITCH_LOGIC(absorption, eval, sample) {                                      \
            float unused_pdf;                                                                     \
            return throughput * eval(inst, normal, wi, wo, unused_pdf);                           \
        }

        MAT_DO_SWITCH(material)
        #undef MAT_SWITCH_LOGIC
    }

    return vec3(0.0);
}

vec3 gather_photons_in_sphere(vec3 point, vec3 wo, vec3 normal, uint material, uint inst) {
    cell_t cell = cell_for_point(point);

    vec3 d = sign(fract(point / integrator.cell_size) - vec3(0.5));

    vec3 estimate = vec3(0.0);

    estimate += get_photon(cell + vec3(0.0, 0.0, 0.0), point, material, inst, normal, wo);
    estimate += get_photon(cell + vec3(0.0, 0.0, d.z), point, material, inst, normal, wo);
    estimate += get_photon(cell + vec3(0.0, d.y, 0.0), point, material, inst, normal, wo);
    estimate += get_photon(cell + vec3(0.0, d.y, d.z), point, material, inst, normal, wo);
    estimate += get_photon(cell + vec3(d.x, 0.0, 0.0), point, material, inst, normal, wo);
    estimate += get_photon(cell + vec3(d.x, 0.0, d.z), point, material, inst, normal, wo);
    estimate += get_photon(cell + vec3(d.x, d.y, 0.0), point, material, inst, normal, wo);
    estimate += get_photon(cell + vec3(d.x, d.y, d.z), point, material, inst, normal, wo);

    return estimate / (M_PI * integrator.search_radius_squared);
}

vec3 gather_photons(ray_t ray, quasi_t quasi) {
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
            vec3 light = mis ? env_sample_light(mis_wi, light_pdf, quasi) : vec3(0.0);

            #define MAT_SWITCH_LOGIC(absorption, eval, sample) {                                  \
                throughput *= absorption(mat_inst, inside, traversal.range.y);                    \
                                                                                                  \
                if (light_pdf != 0.0) {                                                           \
                    mis_f = eval(mat_inst, normal, mis_wi, -ray.dir, mis_material_pdf)            \
                          * abs(dot(mis_wi, normal)) * throughput;                                \
                }                                                                                 \
                                                                                                  \
                f = sample(mat_inst, normal, wi, -ray.dir, material_pdf, quasi);                  \
            }

            MAT_DO_SWITCH(material)
            #undef MAT_SWITCH_LOGIC

            if (!is_receiver) {
                float q = max(0.0, 1.0 - luminance(throughput * f) / luminance(throughput));

                if (quasi_sample(quasi) < q) {
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

                vec3 li = gather_photons_in_sphere(ray.org, -ray.dir, normal, material, mat_inst);
                radiance += li / integrator.photons_for_pass; // normalize the photon contribution

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
    uint seed = (uint(gl_FragCoord.x) << 16U) + uint(gl_FragCoord.y);

    quasi_t quasi = quasi_init(decorrelate_sample(seed), integrator.current_pass);

    ray_t ray;
    evaluate_primary_ray(ray.org, ray.dir, quasi);

    radiance_estimate = vec4(gather_photons(ray, quasi), 1.0);
}
