// requires-define MATERIAL_DATA_LEN

#include <common.glsl>

#include <quasi.glsl>

struct Parameter {
    vec3 base;
    float layer;
    vec3 scale;
    float stochastic_scale;

    float uv_rotation;
    float uv_scale;
    vec2 uv_offset;
};

layout (std140) uniform Material {
    Parameter data[MATERIAL_DATA_LEN];
} material_buffer;

uniform sampler2DArray material_textures;

vec3 getTriPlanarBlend(vec3 _wNorm){
	vec3 blending = abs( _wNorm );
    blending = pow(blending, vec3(16.0));
	blending = normalize(max(blending, 1e-6)); // Force weights to sum to 1.0
	float b = (blending.x + blending.y + blending.z);
	blending /= vec3(b, b, b);
	return blending;
}

// Adapted from https://www.shadertoy.com/view/MdyfDV
vec3 sample_texture_stochastic(float layer, vec2 uv, float scale) {
    vec2 V = vec2(uv.x - 0.57735 * uv.y, 1.1547 * uv.y);
    vec2 I = floor(V);

    vec3 F = vec3(V - I, 0.0);
    F.z = 1.0 - F.x - F.y;

    #define rnd22(p)   fract(sin((p) * mat2(127.1,311.7,269.5,183.3) )*43758.5453)

    #define C(X) textureLod(material_textures, vec3(uv - scale * (X), layer), 0.0).xyz

    vec3 cdx = C(rnd22(I + vec2(1.0, 0.0)));
    vec3 cdy = C(rnd22(I + vec2(0.0, 1.0)));

    vec3 c = C((F.z > 0.0) ? rnd22(I) : rnd22(I + 1.0));

    return clamp(F.z > 0.0 ? F.x * cdx + F.y * cdy + F.z * c
                           : (1.0 - F.x) * cdy + (1.0 - F.y) * cdx - F.z * c, 0.0, 1.0);
}

vec3 sample_texture(float layer, vec2 uv) {
    return textureLod(material_textures, vec3(uv, layer), 0.0).xyz;
}

vec3 mat_param_vec3(uint inst, vec3 normal, vec3 p) {
    Parameter param = material_buffer.data[inst];

    if (param.layer < 0.0 || param.scale.xyz == vec3(0.0)) {
        return param.base.xyz; // texture absent/irrelevant
    }
    
    vec3 yz_sample, xz_sample, xy_sample;

    if (param.stochastic_scale > 0.0) {
        yz_sample = sample_texture_stochastic(param.layer, param.uv_offset + param.uv_scale * p.yz, param.stochastic_scale);
        xz_sample = sample_texture_stochastic(param.layer, param.uv_offset + param.uv_scale * p.xz, param.stochastic_scale);
        xy_sample = sample_texture_stochastic(param.layer, param.uv_offset + param.uv_scale * p.xy, param.stochastic_scale);
    } else {
        yz_sample = sample_texture(param.layer, param.uv_offset + param.uv_scale * p.yz);
        xz_sample = sample_texture(param.layer, param.uv_offset + param.uv_scale * p.xz);
        xy_sample = sample_texture(param.layer, param.uv_offset + param.uv_scale * p.xy);
    }

    /*vec3 yz_sample = sample_texture(param.texture, param.uv_offset + param.uv_scale * p.yz);
    vec3 xz_sample = sample_texture(param.texture, param.uv_offset + param.uv_scale * p.xz);
    vec3 xy_sample = sample_texture(param.texture, param.uv_offset + param.uv_scale * p.xy);*/

    vec3 triplanar_weights = getTriPlanarBlend(normal);

    return param.base.xyz + param.scale.xyz * (yz_sample * triplanar_weights.x
                                            +  xz_sample * triplanar_weights.y
                                            +  xy_sample * triplanar_weights.z);
}

float mat_param_float(uint inst, vec3 normal, vec3 p) {
    return luminance(mat_param_vec3(inst, normal, p));
}

// TODO: find a better way for this; evaluating these parameters is now potentially expensive
// but the macros make them look like cheap lookups. Probably just need to rename them slightly?

// == LAMBERTIAN =================================================================================
#define MAT_LAMBERTIAN_ALBEDO                               mat_param_vec3(inst +  0U, normal, p)
// == IDEAL REFLECTION ===========================================================================
#define MAT_IDEAL_REFLECTION_REFLECTANCE                    mat_param_vec3(inst +  0U, normal, p)
// == IDEAL REFRACTION ===========================================================================
#define MAT_IDEAL_REFRACTION_TRANSMITTANCE                  mat_param_vec3(inst +  0U, normal, p)
// == PHONG ======================================================================================
#define MAT_PHONG_ALBEDO                                    mat_param_vec3(inst +  0U, normal, p)
#define MAT_PHONG_EXPONENT                                  mat_param_float(inst +  1U, normal, p)
// == DIELECTRIC =================================================================================
#define MAT_DIELECTRIC_BASE_COLOR                           mat_param_vec3(inst +  0U, normal, p)
// == OREN-NAYAR =================================================================================
#define MAT_OREN_NAYAR_ALBEDO                               mat_param_vec3(inst +  0U, normal, p)
#define MAT_OREN_NAYAR_ROUGHNESS                            mat_param_float(inst +  1U, normal, p)

// == LAMBERTIAN BSDF ============================================================================

vec3 mat_lambertian_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf, vec3 p) {
    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_LAMBERTIAN_ALBEDO / M_PI);
}

vec3 mat_lambertian_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi, vec3 p) {
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

vec3 mat_ideal_reflection_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf, vec3 p) {
    return pdf = 0.0, vec3(0.0);
}

vec3 mat_ideal_reflection_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi, vec3 p) {
    pdf = 1.0;
    wi = reflect(-wo, normal);

    return MAT_IDEAL_REFLECTION_REFLECTANCE;
}

// == IDEAL REFRACTION BSDF ======================================================================

vec3 mat_ideal_refraction_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf, vec3 p) {
    pdf = 0.0;

    return vec3(0.0);
}

vec3 mat_ideal_refraction_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi, vec3 p) {
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

vec3 mat_phong_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf, vec3 p) {
    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    float exponent = MAT_PHONG_EXPONENT;
    vec3 albedo = MAT_PHONG_ALBEDO;

    float cos_alpha = pow(max(0.0, dot(reflect(-wo, normal), wi)), exponent);

    pdf = cos_alpha * (exponent + 1.0) / M_2PI;

    return albedo * (exponent + 2.0) / M_2PI * cos_alpha / max(1e-6, wi_n);
}

vec3 mat_phong_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi, vec3 p) {
    float u1 = quasi_sample(quasi);
    float u2 = quasi_sample(quasi);

    float exponent = MAT_PHONG_EXPONENT;
    vec3 albedo = MAT_PHONG_ALBEDO;

    float phi = M_2PI * u1;
    float theta = acos(pow(u2, 1.0 / (exponent + 1.0)));

    vec3 ideal = reflect(-wo, normal);

    wi = rotate(to_spherical(phi, theta), ideal);

    if (dot(wi, normal) <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    float cos_alpha = pow(max(0.0, dot(ideal, wi)), exponent);

    pdf = cos_alpha * (exponent + 1.0) / M_2PI;

    return albedo * (exponent + 2.0) / (exponent + 1.0);
}

// == DIELECTRIC BSDF ============================================================================

vec3 mat_dielectric_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf, vec3 p) {
    return pdf = 0.0, vec3(0.0);
}

vec3 mat_dielectric_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi, vec3 p) {
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

float oren_nayar_term(float wi_n, float wo_n, vec3 wi, vec3 wo, vec3 normal, float roughness) {
    float roughness2 = roughness * roughness;

    float a = 1.0 - 0.5 * roughness2 / (roughness2 + 0.33);
    float b = 0.45 * roughness2 / (roughness2 + 0.09);

    vec3 wi_proj = normalize(wi - normal * wi_n);
    vec3 wo_proj = normalize(wo - normal * wo_n);

    float theta_i = acos(wi_n);
    float theta_o = acos(wo_n);

    return a + b * max(0.0, dot(wi_proj, wo_proj)) * sin(max(theta_i, theta_o))
                                                   * tan(min(theta_i, theta_o));
}

vec3 mat_oren_nayar_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf, vec3 p) {
    float wi_n = dot(wi, normal);
    float wo_n = dot(wo, normal);

    if (wi_n <= 0.0 || wo_n <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return vec3(MAT_OREN_NAYAR_ALBEDO / M_PI) * oren_nayar_term(wi_n, wo_n, wi, wo, normal, MAT_OREN_NAYAR_ROUGHNESS);
}

vec3 mat_oren_nayar_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi, vec3 p) {
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

    return vec3(MAT_OREN_NAYAR_ALBEDO) * oren_nayar_term(wi_n, wo_n, wi, wo, normal, MAT_OREN_NAYAR_ROUGHNESS);
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
