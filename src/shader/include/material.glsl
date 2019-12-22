// requires-define MATERIAL_DATA_LEN

#include <common.glsl>

#include <quasi.glsl>

layout (std140) uniform Material {
    vec4 data[MATERIAL_DATA_LEN];
} material_buffer;

uniform sampler2D normal_map;

vec3 getTriPlanarBlend(vec3 _wNorm){
	vec3 blending = abs( _wNorm );
    blending = pow(blending, vec3(128.0));
	blending = normalize(max(blending, 1e-6)); // Force weights to sum to 1.0
	float b = (blending.x + blending.y + blending.z);
	blending /= vec3(b, b, b);
	return blending;
}

vec3 get_normal(vec3 world_normal, vec3 world_pos) {
    vec3 blending = getTriPlanarBlend(world_normal);

    vec3 xaxis = textureLod(normal_map, world_pos.yz * 0.2, 0.0).rbg * 2.0 - 1.0;
	vec3 yaxis = textureLod(normal_map, world_pos.xz * 0.2, 0.0).rbg * 2.0 - 1.0;
	vec3 zaxis = textureLod(normal_map, world_pos.xy * 0.2, 0.0).rbg * 2.0 - 1.0;

    xaxis.xz *= world_normal.x < 0.0 ? +1.0 : -1.0;
    yaxis.xz *= world_normal.y < 0.0 ? -1.0 : +1.0;
    zaxis.xz *= world_normal.z < 0.0 ? +1.0 : -1.0;

    vec3 normalTex = normalize(xaxis * blending.x + yaxis * blending.y + zaxis * blending.z);

    return rotate(normalTex, world_normal);
}

// == LAMBERTIAN =================================================================================
#define MAT_LAMBERTIAN_ALBEDO                               material_buffer.data[inst +  0U].xyz
// == IDEAL REFLECTION ===========================================================================
#define MAT_IDEAL_REFLECTION_REFLECTANCE                    material_buffer.data[inst +  0U].xyz
// == IDEAL REFRACTION ===========================================================================
#define MAT_IDEAL_REFRACTION_TRANSMITTANCE                  material_buffer.data[inst +  0U].xyz
// == PHONG ======================================================================================
#define MAT_PHONG_EXPONENT                                  material_buffer.data[inst +  0U].w
#define MAT_PHONG_ALBEDO                                    material_buffer.data[inst +  0U].xyz
// == DIELECTRIC =================================================================================
#define MAT_DIELECTRIC_BASE_COLOR                           material_buffer.data[inst +  0U].xyz
// == OREN-NAYAR =================================================================================
#define MAT_OREN_NAYAR_ALBEDO                               material_buffer.data[inst +  0U].xyz
#define MAT_OREN_NAYAR_COEFF_A                              material_buffer.data[inst +  1U].x
#define MAT_OREN_NAYAR_COEFF_B                              material_buffer.data[inst +  1U].y

// == LAMBERTIAN BSDF ============================================================================

vec3 mat_lambertian_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_LAMBERTIAN_ALBEDO / M_PI);
}

vec3 mat_lambertian_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
    float u1 = quasi_sample(quasi);
    float u2 = quasi_sample(quasi);

    float r = sqrt(u1);
    float phi = M_2PI * u2;

    wi = rotate(vec3(r * cos(phi), sqrt(1.0 - u1), r * sin(phi)), normal);

    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_LAMBERTIAN_ALBEDO);
}

// == IDEAL REFLECTION BSDF ======================================================================

vec3 mat_ideal_reflection_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    return pdf = 0.0, vec3(0.0);
}

vec3 mat_ideal_reflection_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
    pdf = 1.0;
    wi = reflect(-wo, normal);

    return MAT_IDEAL_REFLECTION_REFLECTANCE;
}

// == IDEAL REFRACTION BSDF ======================================================================

vec3 mat_ideal_refraction_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    pdf = 0.0;

    return vec3(0.0);
}

vec3 mat_ideal_refraction_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
    pdf = 1.0;

    if (dot(wo, normal) >= 0.0) {
        wi = refract(-wo, normal, n1 / n2);

        if (wi == vec3(0.0)) {
            wi = reflect(-wo, normal);
        }
    } else {
        wi = refract(-wo, -normal, n2 / n1);

        if (wi == vec3(0.0)) {
            wi = reflect(-wo, -normal);
        }
    }

    return MAT_IDEAL_REFRACTION_TRANSMITTANCE;
}

// == PHONG BSDF =================================================================================

vec3 mat_phong_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    float cos_alpha = pow(max(0.0, dot(reflect(-wo, normal), wi)), MAT_PHONG_EXPONENT);

    pdf = cos_alpha * (MAT_PHONG_EXPONENT + 1.0) / M_2PI;

    return MAT_PHONG_ALBEDO * (MAT_PHONG_EXPONENT + 2.0) / M_2PI * cos_alpha / max(1e-6, wi_n);
}

vec3 mat_phong_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
    float u1 = quasi_sample(quasi);
    float u2 = quasi_sample(quasi);

    float phi = M_2PI * u1;
    float theta = acos(pow(u2, 1.0 / (MAT_PHONG_EXPONENT + 1.0)));

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

vec3 mat_dielectric_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    return pdf = 0.0, vec3(0.0);
}

vec3 mat_dielectric_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
    pdf = 1.0;

    float cosI = dot(-wo, normal);

    if (cosI < 0.0) {
        cosI = -cosI;
    } else {
        normal = -normal;
    }

    float eta = n1 / n2;

    float cosT = 1.0 - eta * eta * (1.0 - cosI * cosI);

    if (cosT > 0.0) {
        cosT = sqrt(cosT);

        // Account for change in beam area and wave velocity; the change in wave
        // velocity is only important if a light source exists inside the medium
        // as otherwise the factor is cancelled out as the ray exits the medium.

        float ts = 1.0 / (n1 * cosI + n2 * cosT); // s-polarized fresnel
        float tp = 1.0 / (n1 * cosT + n2 * cosI); // p-polarized fresnel
        float t = 2.0 * (ts * ts + tp * tp) * (n1 * cosI) * (n2 * cosT);

        if (quasi_sample(quasi) < t) {
            wi = (eta * cosI - cosT) * normal - eta * wo;
        } else {
            wi = reflect(-wo, normal);
        }
    } else {
        wi = reflect(-wo, normal);
    }

    return MAT_DIELECTRIC_BASE_COLOR;
}

// == OREN-NAYAR BSDF ============================================================================

float oren_nayar_term(float wi_n, float wo_n, vec3 wi, vec3 wo, vec3 normal, float a, float b) {
    vec3 wi_proj = normalize(wi - normal * wi_n);
    vec3 wo_proj = normalize(wo - normal * wo_n);

    float theta_i = acos(wi_n);
    float theta_o = acos(wo_n);

    return a + b * max(0.0, dot(wi_proj, wo_proj)) * sin(max(theta_i, theta_o))
                                                   * tan(min(theta_i, theta_o));
}

vec3 mat_oren_nayar_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    float wi_n = dot(wi, normal);
    float wo_n = dot(wo, normal);

    if (wi_n <= 0.0 || wo_n <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_OREN_NAYAR_ALBEDO / M_PI) * oren_nayar_term(wi_n, wo_n, wi, wo, normal, MAT_OREN_NAYAR_COEFF_A, MAT_OREN_NAYAR_COEFF_B);
}

vec3 mat_oren_nayar_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
    float u1 = quasi_sample(quasi);
    float u2 = quasi_sample(quasi);

    float r = sqrt(u1);
    float phi = M_2PI * u2;

    wi = rotate(vec3(r * cos(phi), sqrt(1.0 - u1), r * sin(phi)), normal);

    float wi_n = dot(wi, normal);
    float wo_n = dot(wo, normal);

    if (wi_n <= 0.0 || wo_n <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_OREN_NAYAR_ALBEDO) * oren_nayar_term(wi_n, wo_n, wi, wo, normal, MAT_OREN_NAYAR_COEFF_A, MAT_OREN_NAYAR_COEFF_B);
}

#define MAT_IS_RECEIVER(material) \
    ((material & 0x8000U) != 0U)

#define MAT_SAMPLE_EXPLICIT(material) \
    ((material & 0x4000U) != 0U)

// An X-macro for inlining arbitrary code inside a material switch-case, to avoid repetitively
// having to dispatch to specific material functions; it expands the `MAT_SWITCH_LOGIC` macro.

#define MAT_DO_SWITCH(material)                                                                   \
    switch (material & 0x3fffU) {                                                                 \
        case 0U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_lambertian_eval_brdf,                                            \
                             mat_lambertian_sample_brdf)                                          \
            break;                                                                                \
        case 1U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_ideal_reflection_eval_brdf,                                      \
                             mat_ideal_reflection_sample_brdf)                                    \
            break;                                                                                \
        case 2U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_phong_eval_brdf,                                                 \
                             mat_phong_sample_brdf)                                               \
            break;                                                                                \
        case 3U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_ideal_refraction_eval_brdf,                                      \
                             mat_ideal_refraction_sample_brdf)                                    \
            break;                                                                                \
        case 4U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_dielectric_eval_brdf,                                            \
                             mat_dielectric_sample_brdf)                                          \
            break;                                                                                \
        case 5U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_oren_nayar_eval_brdf,                                            \
                             mat_oren_nayar_sample_brdf)                                          \
            break;                                                                                \
    }
