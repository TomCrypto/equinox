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

vec3 get_photon(cell_t cell, vec3 point, uint mat_type, material_t material, vec3 normal, vec3 wo, float n1, float n2) {
    ivec2 coords = hash_entry_for_cell(cell);

    vec4 pos_data = texelFetch(photon_table_pos, coords, 0);
    vec3 position = pos_data.xyz / pos_data.w;

    if (dot(point - position, point - position) <= integrator.search_radius_squared) {
        vec3 photon_wi = 2.0 * texelFetch(photon_table_dir, coords, 0).rgb - 1.0;
        vec3 throughput = 65536.0 * texelFetch(photon_table_sum, coords, 0).rgb;

        #define MAT_SWITCH_LOGIC(LOAD, EVAL, SAMPLE) {                                            \
            float unused_pdf;                                                                     \
            return throughput * EVAL(material, normal, photon_wi, wo, n1, n2, unused_pdf);        \
        }

        MAT_DO_SWITCH(mat_type)
        #undef MAT_SWITCH_LOGIC
    }

    return vec3(0.0);
}

vec3 query_photon_map(vec3 point, vec3 wo, vec3 normal, uint mat_type, material_t material, float n1, float n2) {
    cell_t cell = cell_for_point(point);

    vec3 d = sign(fract(point / integrator.cell_size) - vec3(0.5));

    vec3 estimate = vec3(0.0);

    estimate += get_photon(cell + vec3(0.0, 0.0, 0.0), point, mat_type, material, normal, wo, n1, n2);
    estimate += get_photon(cell + vec3(0.0, 0.0, d.z), point, mat_type, material, normal, wo, n1, n2);
    estimate += get_photon(cell + vec3(0.0, d.y, 0.0), point, mat_type, material, normal, wo, n1, n2);
    estimate += get_photon(cell + vec3(0.0, d.y, d.z), point, mat_type, material, normal, wo, n1, n2);
    estimate += get_photon(cell + vec3(d.x, 0.0, 0.0), point, mat_type, material, normal, wo, n1, n2);
    estimate += get_photon(cell + vec3(d.x, 0.0, d.z), point, mat_type, material, normal, wo, n1, n2);
    estimate += get_photon(cell + vec3(d.x, d.y, 0.0), point, mat_type, material, normal, wo, n1, n2);
    estimate += get_photon(cell + vec3(d.x, d.y, d.z), point, mat_type, material, normal, wo, n1, n2);

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

            uint mat_type = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;
            material_t material;

            bool is_receiver = MAT_IS_RECEIVER(mat_type);

            bool inside = dot(ray.dir, normal) > 0.0;

            float u1 = quasi_sample(quasi);
            float u2 = quasi_sample(quasi);
            float u3 = quasi_sample(quasi);
            float u4 = quasi_sample(quasi);
            float u5 = quasi_sample(quasi);

            float n1, n2;

            throughput *= medium_absorption(traversal.hit.x >> 16U, inside,
                                            traversal.range.y, n1, n2);

            mis = MAT_SAMPLE_EXPLICIT(mat_type) && (bounce != integrator.max_gather_bounces - 1U)
                                                && !inside;

            vec3 wi, f, mis_f, mis_wi;
            float mis_material_pdf;

            light_pdf = 0.0;
            vec3 light = mis ? env_sample_light(mis_wi, light_pdf, u1, u2) : vec3(0.0);

            #define MAT_SWITCH_LOGIC(LOAD, EVAL, SAMPLE) {                                        \
                LOAD(mat_inst, normal, ray.org, material);                                        \
                                                                                                  \
                if (light_pdf != 0.0) {                                                           \
                    mis_f = EVAL(material, normal, mis_wi, -ray.dir, n1, n2, mis_material_pdf)    \
                          * abs(dot(mis_wi, normal)) * throughput;                                \
                }                                                                                 \
                                                                                                  \
                f = SAMPLE(material, normal, wi, -ray.dir, n1, n2, material_pdf, u3, u4);         \
            }

            MAT_DO_SWITCH(mat_type)
            #undef MAT_SWITCH_LOGIC

            if (!is_receiver) {
                float q = max(0.0, 1.0 - luminance(throughput * f) / luminance(throughput));

                if (u5 < q) {
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
                if (mis && material_pdf != 0.0) {
                    // Finish the MIS direct light sampling procedure we started earlier; this
                    // is done to ensure that the MIS weights result in an unbiased estimator.

                    if (!is_ray_occluded(make_ray(ray.org, wi, normal), 1.0 / 0.0)) {
                        vec3 light = env_eval_light(wi, light_pdf);

                        if (light_pdf != 0.0) {
                            radiance += throughput * f * light * power_heuristic(material_pdf, light_pdf);
                        }
                    }
                }

                vec3 li = query_photon_map(ray.org, -ray.dir, normal, mat_type, material, n1, n2);
                radiance += throughput * li / integrator.photons_for_pass; // SPPM photon estimate

                return radiance;
            }

            ray = make_ray(ray.org, wi, normal);
        } else {
            // If we began an MIS direct light sampling procedure in the previous bounce, finish
            // it now; the ray was clearly not occluded so accumulate the light with MIS weight.

            vec3 light = env_eval_light(ray.dir, light_pdf);

            if (mis && material_pdf != 0.0 && light_pdf != 0.0) {
                radiance += throughput * light * power_heuristic(material_pdf, light_pdf);
            } else {
                radiance += throughput * light;
            }

            return radiance;
        }
    }

    return radiance;
}

void main() {
    uint seed = (uint(gl_FragCoord.x) << 16U) + uint(gl_FragCoord.y);

    quasi_t quasi = quasi_init(integrator.current_pass, decorrelate_sample(seed));

    ray_t ray = evaluate_camera_ray(gl_FragCoord.xy - 0.5, quasi);

    radiance_estimate = vec4(gather_photons(ray, quasi), 1.0);
}
