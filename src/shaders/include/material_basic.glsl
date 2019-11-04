#include <common.glsl>
#include <random.glsl>

layout (std140) uniform Material {
    vec4 data[MATERIAL_DATA_COUNT];
} material_buffer;

#define MAT_FLAG_ALLOW_MIS                                                           (1U << 15U)
#define MAT_FLAG_MASK                                                                    0xff00U

#define RAY_FLAG_OUTSIDE                                                             (1U <<  0U)
#define RAY_FLAG_TRANSMIT                                                            (1U <<  1U)
#define RAY_FLAG_EXTINCT                                                             (1U <<  2U)
#define RAY_FLAG_ENVMAP_SAMPLED                                                      (1U <<  3U)
#define RAY_FLAG_MASK                                                                    0x00ffU

#define MAT_PROP_DIFFUSE_BSDF                                                        (1U <<  8U)
#define MAT_PROP_GLOSSY_BSDF                                                         (1U <<  9U)
#define MAT_PROP_DELTA_BSDF                                                          (1U << 10U)

// == LAMBERTIAN =================================================================================
#define MAT_LAMBERTIAN_ALBEDO                               material_buffer.data[inst +  0U].xyz
// == IDEAL REFLECTION ===========================================================================
#define MAT_IDEAL_REFLECTION_REFLECTANCE                    material_buffer.data[inst +  0U].xyz
// == IDEAL REFRACTION ===========================================================================
#define MAT_IDEAL_REFRACTION_TRANSMITTANCE                  material_buffer.data[inst +  0U].xyz
#define MAT_IDEAL_REFRACTION_IOR                            material_buffer.data[inst +  0U].w
// == PHONG ======================================================================================
#define MAT_PHONG_EXPONENT                                  material_buffer.data[inst +  0U].w
#define MAT_PHONG_ALBEDO                                    material_buffer.data[inst +  0U].xyz
// == DIELECTRIC =================================================================================
#define MAT_DIELECTRIC_INTERNAL_EXTINCTION_COEFFICIENT      material_buffer.data[inst +  0U].xyz
#define MAT_DIELECTRIC_INTERNAL_REFRACTIVE_INDEX            material_buffer.data[inst +  0U].w
#define MAT_DIELECTRIC_EXTERNAL_EXTINCTION_COEFFICIENT      material_buffer.data[inst +  1U].xyz
#define MAT_DIELECTRIC_EXTERNAL_REFRACTIVE_INDEX            material_buffer.data[inst +  1U].w
#define MAT_DIELECTRIC_BASE_COLOR                           material_buffer.data[inst +  2U].xyz
// == OREN-NAYAR =================================================================================
#define MAT_OREN_NAYAR_ALBEDO                               material_buffer.data[inst +  0U].xyz
#define MAT_OREN_NAYAR_COEFF_A                              material_buffer.data[inst +  1U].x
#define MAT_OREN_NAYAR_COEFF_B                              material_buffer.data[inst +  1U].y

// == LAMBERTIAN BSDF ============================================================================

vec3 mat_lambertian_absorption(uint inst, float path_length, inout uint flags) {
    if ((flags & RAY_FLAG_OUTSIDE) == 0U) {
        flags |= RAY_FLAG_EXTINCT;
        return vec3(0.0);
    }

    return vec3(1.0); // TODO: add external extinction coefficients
}

vec3 mat_lambertian_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, out float pdf) {
    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_LAMBERTIAN_ALBEDO / M_PI);
}

vec3 mat_lambertian_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, out float pdf, inout uint flags, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    float r = sqrt(rng.x);
    float phi = M_2PI * rng.y;

    wi = rotate(vec3(r * cos(phi), sqrt(1.0 - rng.x), r * sin(phi)), normal);

    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_LAMBERTIAN_ALBEDO);
}

// == IDEAL REFLECTION BSDF ======================================================================

vec3 mat_ideal_reflection_absorption(uint inst, float path_length, inout uint flags) {
    if ((flags & RAY_FLAG_OUTSIDE) == 0U) {
        flags |= RAY_FLAG_EXTINCT;
        return vec3(0.0);
    }

    return vec3(1.0); // TODO: add external extinction coefficients
}

vec3 mat_ideal_reflection_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, out float pdf) {
    return pdf = 0.0, vec3(0.0);
}

vec3 mat_ideal_reflection_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, out float pdf, inout uint flags, inout random_t random) {
    pdf = 1.0;
    wi = reflect(-wo, normal);

    return MAT_IDEAL_REFLECTION_REFLECTANCE;
}

// == IDEAL REFRACTION BSDF ======================================================================

vec3 mat_ideal_refraction_absorption(uint inst, float path_length, inout uint flags) {
    return vec3(1.0); // TODO: add external extinction coefficients
}

vec3 mat_ideal_refraction_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, out float pdf) {
    pdf = 0.0;

    return vec3(0.0);
}

vec3 mat_ideal_refraction_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, out float pdf, inout uint flags, inout random_t random) {
    pdf = 1.0;

    if (dot(wo, normal) >= 0.0) {
        wi = refract(-wo, normal, 1.0 / MAT_IDEAL_REFRACTION_IOR);

        if (wi == vec3(0.0)) {
            wi = reflect(-wo, normal);
        } else {
            flags |= RAY_FLAG_TRANSMIT;
        }
    } else {
        wi = refract(-wo, -normal, MAT_IDEAL_REFRACTION_IOR);

        if (wi == vec3(0.0)) {
            wi = reflect(-wo, -normal);
        } else {
            flags |= RAY_FLAG_TRANSMIT;
        }
    }

    return MAT_IDEAL_REFRACTION_TRANSMITTANCE;
}

// == PHONG BSDF =================================================================================

vec3 mat_phong_absorption(uint inst, float path_length, inout uint flags) {
    if ((flags & RAY_FLAG_OUTSIDE) == 0U) {
        flags |= RAY_FLAG_EXTINCT;
        return vec3(0.0);
    }

    return vec3(1.0); // TODO: add external extinction coefficients
}

vec3 mat_phong_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, out float pdf) {
    if (dot(wi, normal) <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    float cos_alpha = pow(max(0.0, dot(reflect(-wo, normal), wi)), MAT_PHONG_EXPONENT);

    pdf = cos_alpha * (MAT_PHONG_EXPONENT + 1.0) / M_2PI;

    return MAT_PHONG_ALBEDO * (MAT_PHONG_EXPONENT + 2.0) / M_2PI * cos_alpha;
}

vec3 mat_phong_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, out float pdf, inout uint flags, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    float phi = M_2PI * rng.x;
    float theta = acos(pow(rng.y, 1.0 / (MAT_PHONG_EXPONENT + 1.0)));

    vec3 ideal = reflect(-wo, normal);

    wi = rotate(to_spherical(phi, theta), ideal);

    if (dot(wi, normal) <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    float cos_alpha = pow(max(0.0, dot(ideal, wi)), MAT_PHONG_EXPONENT);

    pdf = cos_alpha * (MAT_PHONG_EXPONENT + 1.0) / M_2PI;

    return MAT_PHONG_ALBEDO * (MAT_PHONG_EXPONENT + 2.0) / (MAT_PHONG_EXPONENT + 1.0);
}

// == DIELECTRIC BSDF ============================================================================

vec3 mat_dielectric_absorption(uint inst, float path_length, inout uint flags) {
    vec3 extinction;

    if ((flags & RAY_FLAG_OUTSIDE) == 0U) {
        extinction = MAT_DIELECTRIC_INTERNAL_EXTINCTION_COEFFICIENT
                   * MAT_DIELECTRIC_INTERNAL_REFRACTIVE_INDEX;
    } else {
        extinction = MAT_DIELECTRIC_EXTERNAL_EXTINCTION_COEFFICIENT
                   * MAT_DIELECTRIC_EXTERNAL_REFRACTIVE_INDEX;
    }

    return exp(-extinction * M_2PI * path_length / vec3(685e-9, 530e-9, 470e-9));
}

vec3 mat_dielectric_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, out float pdf) {
    return pdf = 0.0, vec3(0.0);
}

vec3 mat_dielectric_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, out float pdf, inout uint flags, inout random_t random) {
    pdf = 1.0;

    float n1, n2, cosI = dot(wo, normal);

    if (cosI > 0.0) {
        n1 = MAT_DIELECTRIC_EXTERNAL_REFRACTIVE_INDEX;
        n2 = MAT_DIELECTRIC_INTERNAL_REFRACTIVE_INDEX;
    } else {
        n1 = MAT_DIELECTRIC_INTERNAL_REFRACTIVE_INDEX;
        n2 = MAT_DIELECTRIC_EXTERNAL_REFRACTIVE_INDEX;

        normal = -normal;
        cosI = -cosI;
    }

    float eta = n1 / n2;

    float cosT = 1.0 - eta * eta * (1.0 - cosI * cosI);

    if (cosT > 0.0) {
        cosT = sqrt(cosT);

        float rs = (n1 * cosI - n2 * cosT) / (n1 * cosI + n2 * cosT);
        float rp = (n1 * cosT - n2 * cosI) / (n1 * cosT + n2 * cosI);
        float r = (rs * rs + rp * rp) * 0.5; // unpolarized lighting

#ifdef SPLIT
        if (rand_uniform_vec2(random).x < 0.5) {
#else
        if (rand_uniform_vec2(random).x < r) {
#endif
            wi = reflect(-wo, normal);
#ifdef SPLIT
            return MAT_DIELECTRIC_BASE_COLOR * r / 0.5;
#endif
        } else {
            wi = (eta * cosI - cosT) * normal - eta * wo;
            flags |= RAY_FLAG_TRANSMIT;

            // Account for change in beam area and wave velocity; the change in wave
            // velocity is only important if a light source exists inside the medium
            // as otherwise the factor is cancelled out as the ray exits the medium.

#ifdef SPLIT
            return MAT_DIELECTRIC_BASE_COLOR * cosT / (cosI * eta) * (1.0 - r) / 0.5;
#else
            return MAT_DIELECTRIC_BASE_COLOR * cosT / (cosI * eta);
#endif
        }
    } else {
        wi = reflect(-wo, normal);
    }

    return MAT_DIELECTRIC_BASE_COLOR;
}

// == OREN-NAYAR BSDF ============================================================================

vec3 mat_oren_nayar_absorption(uint inst, float path_length, inout uint flags) {
    if ((flags & RAY_FLAG_OUTSIDE) == 0U) {
        flags |= RAY_FLAG_EXTINCT;
        return vec3(0.0);
    }

    return vec3(1.0); // TODO: add external extinction coefficients
}

float oren_nayar_term(float wi_n, float wo_n, vec3 wi, vec3 wo, vec3 normal, float a, float b) {
    vec3 wi_proj = normalize(wi - normal * wi_n);
    vec3 wo_proj = normalize(wo - normal * wo_n);

    float theta_i = acos(wi_n);
    float theta_o = acos(wo_n);

    return a + b * max(0.0, dot(wi_proj, wo_proj)) * sin(max(theta_i, theta_o))
                                                   * tan(min(theta_i, theta_o));
}

vec3 mat_oren_nayar_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, out float pdf) {
    float wi_n = dot(wi, normal);
    float wo_n = dot(wo, normal);

    if (wi_n <= 0.0 || wo_n <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_OREN_NAYAR_ALBEDO / M_PI) * oren_nayar_term(wi_n, wo_n, wi, wo, normal, MAT_OREN_NAYAR_COEFF_A, MAT_OREN_NAYAR_COEFF_B);
}

vec3 mat_oren_nayar_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, out float pdf, inout uint flags, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    float r = sqrt(rng.x);
    float phi = M_2PI * rng.y;

    wi = rotate(vec3(r * cos(phi), sqrt(1.0 - rng.x), r * sin(phi)), normal);

    float wi_n = dot(wi, normal);
    float wo_n = dot(wo, normal);

    if (wi_n <= 0.0 || wo_n <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_OREN_NAYAR_ALBEDO) * oren_nayar_term(wi_n, wo_n, wi, wo, normal, MAT_OREN_NAYAR_COEFF_A, MAT_OREN_NAYAR_COEFF_B);
}

bool mat_is_diffuse(uint material) {
    switch (material) {
        case 0U:
            return true;
        case 1U:
            return false;
        case 2U:
            return false;
        case 3U:
            return false;
        case 4U:
            return false;
        case 5U:
            return true;
        default:
            return false;
    }
}

vec3 mat_eval_brdf(uint material, uint inst, vec3 normal, vec3 wi, vec3 wo, out float pdf) {
    switch (material) {
        case 0U:
            return mat_lambertian_eval_brdf(inst, normal, wi, wo, pdf);
        case 1U:
            return mat_ideal_reflection_eval_brdf(inst, normal, wi, wo, pdf);
        case 2U:
            return mat_phong_eval_brdf(inst, normal, wi, wo, pdf);
        case 3U:
            return mat_ideal_refraction_eval_brdf(inst, normal, wi, wo, pdf);
        case 4U:
            return mat_dielectric_eval_brdf(inst, normal, wi, wo, pdf);
        case 5U:
            return mat_oren_nayar_eval_brdf(inst, normal, wi, wo, pdf);
        default:
            return vec3(0.0);
    }
}
