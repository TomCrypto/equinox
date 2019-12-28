// requires-define MATERIAL_DATA_LEN

#include <common.glsl>

#include <quasi.glsl>

struct GeometryParameter {
    vec3 base;
    uint layer;
    vec3 factor;
    float contrast;

    float uv_rotation;
    float uv_scale;
    vec2 uv_offset;
};

layout (std140) uniform Material {
    GeometryParameter data[MATERIAL_DATA_LEN];
} material_buffer;

uniform sampler2DArray material_textures;

vec3 triplanar_weights(vec3 normal) {
    vec3 tri_weight = pow(abs(normal), vec3(12.0));
	return tri_weight / dot(tri_weight, vec3(1.0));
}

// Adapted from https://www.shadertoy.com/view/MdyfDV
vec3 sample_texture_stochastic(float layer, vec2 uv) {
    vec2 V = 4.0 * vec2(uv.x - 0.57735 * uv.y, 1.1547 * uv.y);
    vec2 I = floor(V);

    vec3 F = vec3(V - I, 0.0);
    F.z = 1.0 - F.x - F.y;

    #define rnd22(p) fract(sin((p) * mat2(127.1, 311.7, 269.5, 183.3) ) * 43758.5453)

    #define C(X) textureLod(material_textures, vec3(uv - (X), layer), 0.0).xyz

    vec3 cdx = C(rnd22(I + vec2(1.0, 0.0)));
    vec3 cdy = C(rnd22(I + vec2(0.0, 1.0)));

    vec3 c = C((F.z > 0.0) ? rnd22(I) : rnd22(I + 1.0));

    return clamp(F.z > 0.0 ? F.x * cdx + F.y * cdy + F.z * c
                           : (1.0 - F.x) * cdy + (1.0 - F.y) * cdx - F.z * c,
                 0.0, 1.0);
}

vec3 sample_texture_wraparound(float layer, vec2 uv) {
    return textureLod(material_textures, vec3(uv, layer), 0.0).xyz;
}

vec3 mat_param_vec3(uint inst, vec3 normal, vec3 p) {
    GeometryParameter param = material_buffer.data[inst];

    if (param.layer == 0xffffffffU || param.factor.xyz == vec3(0.0)) {
        return param.base.xyz; // the texture is absent or irrelevant
    }

    float s = param.uv_scale * sin(param.uv_rotation);
	float c = param.uv_scale * cos(param.uv_rotation);
	mat3x2 xfm = mat3x2(c, -s, s, c, param.uv_offset);

    // Offset all triplanar coordinates slightly based on the normal direction
    // in order to randomize e.g. parallel sides of a box or a sheet of glass.

    vec2 yz_uv = xfm * vec3(p.yz + (normal.x > 0.0 ? 0.0 : 17.4326), 1.0);
    vec2 xz_uv = xfm * vec3(p.xz + (normal.y > 0.0 ? 0.0 : 13.8193), 1.0);
    vec2 xy_uv = xfm * vec3(p.xy + (normal.z > 0.0 ? 0.0 : 15.2175), 1.0);

    vec3 yz_sample, xz_sample, xy_sample;
    vec3 tri = triplanar_weights(normal);

    float vert_layer = float(param.layer & 0xffffU);
    float horz_layer = float(param.layer >> 16U);

    if (param.contrast > 0.0) {
        yz_sample = tri.x < 1e-4 ? vec3(0.0) : sample_texture_stochastic(vert_layer, yz_uv);
        xz_sample = tri.y < 1e-4 ? vec3(0.0) : sample_texture_stochastic(horz_layer, xz_uv);
        xy_sample = tri.z < 1e-4 ? vec3(0.0) : sample_texture_stochastic(vert_layer, xy_uv);
    } else {
        yz_sample = tri.x < 1e-4 ? vec3(0.0) : sample_texture_wraparound(vert_layer, yz_uv);
        xz_sample = tri.y < 1e-4 ? vec3(0.0) : sample_texture_wraparound(horz_layer, xz_uv);
        xy_sample = tri.z < 1e-4 ? vec3(0.0) : sample_texture_wraparound(vert_layer, xy_uv);
    }

    yz_sample = 0.5 + (yz_sample - 0.5) * abs(param.contrast);
    xz_sample = 0.5 + (xz_sample - 0.5) * abs(param.contrast);
    xy_sample = 0.5 + (xy_sample - 0.5) * abs(param.contrast);

    return param.base.xyz + param.factor.xyz * (yz_sample * tri.x
                                             +  xz_sample * tri.y
                                             +  xy_sample * tri.z);
}

float mat_param_float(uint inst, vec3 normal, vec3 p) {
    return luminance(mat_param_vec3(inst, normal, p));
}

// Prior to using a material, its parameters must be loaded as a function of normal and shading
// point, which may involve many texture fetches. These are cached into the `material_t` struct
// below so that the actual BRDF evaluation logic never has to do any texture fetches directly.

struct material_t {
    vec4 data[1];
};

// == LAMBERTIAN =================================================================================
#define MAT_LAMBERTIAN_ALBEDO                                                material.data[0].xyz
// == IDEAL REFLECTION ===========================================================================
#define MAT_IDEAL_REFLECTION_REFLECTANCE                                     material.data[0].xyz
// == IDEAL REFRACTION ===========================================================================
#define MAT_IDEAL_REFRACTION_TRANSMITTANCE                                   material.data[0].xyz
// == PHONG ======================================================================================
#define MAT_PHONG_ALBEDO                                                     material.data[0].xyz
#define MAT_PHONG_EXPONENT                                                   material.data[0].w
// == DIELECTRIC =================================================================================
#define MAT_DIELECTRIC_BASE_COLOR                                            material.data[0].xyz
// == OREN-NAYAR =================================================================================
#define MAT_OREN_NAYAR_ALBEDO                                                material.data[0].xyz
#define MAT_OREN_NAYAR_ROUGHNESS                                             material.data[0].w

// == LAMBERTIAN BRDF ============================================================================

void mat_lambertian_load(uint inst, vec3 normal, vec3 point, out material_t material) {
    MAT_LAMBERTIAN_ALBEDO = clamp(mat_param_vec3(inst + 0U, normal, point), 0.0, 1.0);
}

vec3 mat_lambertian_eval(material_t material, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return MAT_LAMBERTIAN_ALBEDO / M_PI;
}

vec3 mat_lambertian_sample(material_t material, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
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

    return MAT_LAMBERTIAN_ALBEDO;
}

// == IDEAL REFLECTION BRDF ======================================================================

void mat_ideal_reflection_load(uint inst, vec3 normal, vec3 point, out material_t material) {
    MAT_IDEAL_REFLECTION_REFLECTANCE = clamp(mat_param_vec3(inst + 0U, normal, point), 0.0, 1.0);
}

vec3 mat_ideal_reflection_eval(material_t material, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    return pdf = 0.0, vec3(0.0);
}

vec3 mat_ideal_reflection_sample(material_t material, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
    pdf = 1.0;
    wi = reflect(-wo, normal);

    return MAT_IDEAL_REFLECTION_REFLECTANCE;
}

// == IDEAL REFRACTION BSDF ======================================================================

void mat_ideal_refraction_load(uint inst, vec3 normal, vec3 point, out material_t material) {
    MAT_IDEAL_REFRACTION_TRANSMITTANCE = clamp(mat_param_vec3(inst + 0U, normal, point), 0.0, 1.0);
}

vec3 mat_ideal_refraction_eval(material_t material, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    pdf = 0.0;

    return vec3(0.0);
}

vec3 mat_ideal_refraction_sample(material_t material, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
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

// == PHONG BRDF =================================================================================

void mat_phong_load(uint inst, vec3 normal, vec3 point, out material_t material) {
    MAT_PHONG_ALBEDO = clamp(mat_param_vec3(inst + 0U, normal, point), 0.0, 1.0);
    MAT_PHONG_EXPONENT = max(mat_param_float(inst + 1U, normal, point), 1.0);
}

vec3 mat_phong_eval(material_t material, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    float wi_n = dot(wi, normal);

    if (wi_n <= 0.0 || dot(wo, normal) <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    float cos_alpha = pow(max(0.0, dot(reflect(-wo, normal), wi)), MAT_PHONG_EXPONENT);

    pdf = cos_alpha * (MAT_PHONG_EXPONENT + 1.0) / M_2PI;

    return MAT_PHONG_ALBEDO * (MAT_PHONG_EXPONENT + 2.0) / M_2PI * cos_alpha / max(1e-6, wi_n);
}

vec3 mat_phong_sample(material_t material, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
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

void mat_dielectric_load(uint inst, vec3 normal, vec3 point, out material_t material) {
    MAT_DIELECTRIC_BASE_COLOR = clamp(mat_param_vec3(inst + 0U, normal, point), 0.0, 1.0);
}

vec3 mat_dielectric_eval(material_t material, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    return pdf = 0.0, vec3(0.0);
}

vec3 mat_dielectric_sample(material_t material, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
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

// == OREN-NAYAR BRDF ============================================================================

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

void mat_oren_nayar_load(uint inst, vec3 normal, vec3 point, out material_t material) {
    MAT_OREN_NAYAR_ALBEDO = clamp(mat_param_vec3(inst + 0U, normal, point), 0.0, 1.0);
    MAT_OREN_NAYAR_ROUGHNESS = clamp(mat_param_float(inst + 1U, normal, point), 0.0, 1.0);
}

vec3 mat_oren_nayar_eval(material_t material, vec3 normal, vec3 wi, vec3 wo, float n1, float n2, out float pdf) {
    float wi_n = dot(wi, normal);
    float wo_n = dot(wo, normal);

    if (wi_n <= 0.0 || wo_n <= 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = wi_n / M_PI;

    return MAT_OREN_NAYAR_ALBEDO / M_PI * oren_nayar_term(wi_n, wo_n, wi, wo, normal, MAT_OREN_NAYAR_ROUGHNESS);
}

vec3 mat_oren_nayar_sample(material_t material, vec3 normal, out vec3 wi, vec3 wo, float n1, float n2, out float pdf, inout quasi_t quasi) {
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

    return MAT_OREN_NAYAR_ALBEDO * oren_nayar_term(wi_n, wo_n, wi, wo, normal, MAT_OREN_NAYAR_ROUGHNESS);
}

#define MAT_IS_RECEIVER(mat_type) \
    ((mat_type & 0x8000U) != 0U)

#define MAT_SAMPLE_EXPLICIT(mat_type) \
    ((mat_type & 0x4000U) != 0U)

// An X-macro for inlining arbitrary code inside a material switch-case, to avoid repetitively
// having to dispatch to specific material functions; it expands the `MAT_SWITCH_LOGIC` macro.

#define MAT_DO_SWITCH(mat_type)                                                                   \
    switch (mat_type & 0x3fffU) {                                                                 \
        case 0U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_lambertian_load,                                                 \
                             mat_lambertian_eval,                                                 \
                             mat_lambertian_sample)                                               \
            break;                                                                                \
        case 1U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_ideal_reflection_load,                                           \
                             mat_ideal_reflection_eval,                                           \
                             mat_ideal_reflection_sample)                                         \
            break;                                                                                \
        case 2U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_phong_load,                                                      \
                             mat_phong_eval,                                                      \
                             mat_phong_sample)                                                    \
            break;                                                                                \
        case 3U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_ideal_refraction_load,                                           \
                             mat_ideal_refraction_eval,                                           \
                             mat_ideal_refraction_sample)                                         \
            break;                                                                                \
        case 4U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_dielectric_load,                                                 \
                             mat_dielectric_eval,                                                 \
                             mat_dielectric_sample)                                               \
            break;                                                                                \
        case 5U:                                                                                  \
            MAT_SWITCH_LOGIC(mat_oren_nayar_load,                                                 \
                             mat_oren_nayar_eval,                                                 \
                             mat_oren_nayar_sample)                                               \
            break;                                                                                \
    }
