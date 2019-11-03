#include <material_basic.glsl>

#include <environment.glsl>
#include <instance.glsl>

// == HIGH-LEVEL MATERIAL INTERACTION ============================================================

#define MAT_INTERACT(absorption, eval_brdf, sample_brdf, props) {                                 \
    float light_pdf, scatter_pdf;                                                                 \
    vec3 wi;                                                                                      \
                                                                                                  \
    flags |= RAY_FLAG_OUTSIDE * uint(dot(wo, normal) > 0.0);                                      \
                                                                                                  \
    throughput *= absorption(inst, path_length, flags);                                           \
                                                                                                  \
    if ((flags & RAY_FLAG_EXTINCT) != 0U) {                                                       \
        return ray_t(point, normal);                                                              \
    }                                                                                             \
                                                                                                  \
    if ((props & MAT_PROP_DELTA_BSDF) == 0U && (flags & MAT_FLAG_ALLOW_MIS) != 0U) {              \
        vec3 Li = env_sample_light(wi, light_pdf, random), f;                                     \
                                                                                                  \
        if (light_pdf != 0.0) {                                                                   \
            f = eval_brdf(inst, normal, wi, wo, scatter_pdf) * abs(dot(wi, normal));              \
                                                                                                  \
            if (scatter_pdf != 0.0 && !is_ray_occluded(make_ray(point, wi, normal), 1.0 / 0.0)) { \
                radiance += throughput * f * Li * power_heuristic(light_pdf, scatter_pdf);        \
            }                                                                                     \
        }                                                                                         \
                                                                                                  \
        throughput *= sample_brdf(inst, normal, wi, wo, scatter_pdf, flags, random);              \
                                                                                                  \
        if (scatter_pdf != 0.0 && !is_ray_occluded(make_ray(point, wi, normal), 1.0 / 0.0)) {     \
            Li = env_eval_light(wi, light_pdf);                                                   \
                                                                                                  \
            if (light_pdf != 0.0) {                                                               \
                radiance += throughput * Li * power_heuristic(scatter_pdf, light_pdf);            \
            }                                                                                     \
        }                                                                                         \
                                                                                                  \
        flags |= RAY_FLAG_ENVMAP_SAMPLED;                                                         \
    } else {                                                                                      \
        throughput *= sample_brdf(inst, normal, wi, wo, scatter_pdf, flags, random);              \
    }                                                                                             \
                                                                                                  \
    flags = (flags & ~MAT_FLAG_MASK) | props; /* store properties */                              \
    return ray_t(point + PREC * sign(dot(wi, normal)) * normal, wi);                              \
}

ray_t mat_interact(uint material, uint inst, vec3 normal, vec3 wo, vec3 point, float path_length,
                   inout vec3 throughput, inout vec3 radiance, out uint flags, inout random_t random) {
    flags = material & MAT_FLAG_MASK;

    switch (material & ~MAT_FLAG_MASK) {
        case 0U:
            MAT_INTERACT(mat_lambertian_absorption,
                         mat_lambertian_eval_brdf,
                         mat_lambertian_sample_brdf,
                         MAT_PROP_DIFFUSE_BSDF)
        case 1U:
            MAT_INTERACT(mat_ideal_reflection_absorption,
                         mat_ideal_reflection_eval_brdf,
                         mat_ideal_reflection_sample_brdf,
                         MAT_PROP_DELTA_BSDF)
        case 2U:
            MAT_INTERACT(mat_phong_absorption,
                         mat_phong_eval_brdf,
                         mat_phong_sample_brdf,
                         MAT_PROP_GLOSSY_BSDF)
        case 3U:
            MAT_INTERACT(mat_ideal_refraction_absorption,
                         mat_ideal_refraction_eval_brdf,
                         mat_ideal_refraction_sample_brdf,
                         MAT_PROP_DELTA_BSDF)
        case 4U:
            MAT_INTERACT(mat_dielectric_absorption,
                         mat_dielectric_eval_brdf,
                         mat_dielectric_sample_brdf,
                         MAT_PROP_DELTA_BSDF)
        case 5U:
            MAT_INTERACT(mat_oren_nayar_absorption,
                         mat_oren_nayar_eval_brdf,
                         mat_oren_nayar_sample_brdf,
                         MAT_PROP_DIFFUSE_BSDF)
        default:
            flags |= RAY_FLAG_EXTINCT;
            return ray_t(point, normal);
    }
}

#undef MAT_INTERACT
