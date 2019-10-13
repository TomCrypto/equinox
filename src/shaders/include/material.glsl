#include <common.glsl>
#include <random.glsl>

layout (std140) uniform Material {
    vec4 data[MATERIAL_DATA_COUNT];
} material_buffer;

// Material parameter packing

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

// Material BRDF definitions

vec3 mat_lambertian_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo) {
    return vec3(MAT_LAMBERTIAN_ALBEDO / M_PI);
}

vec3 mat_lambertian_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float path_length, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    float r = sqrt(rng.x);
    float phi = M_2PI * rng.y;

    wi = rotate(vec3(r * cos(phi), sqrt(1.0 - rng.x), r * sin(phi)), normal);

    pdf = 1.0;

    return vec3(MAT_LAMBERTIAN_ALBEDO);
}

vec3 mat_ideal_reflection_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo) {
    return vec3(0.0);
}

vec3 mat_ideal_reflection_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float path_length, out float pdf, inout random_t random) {
    pdf = 1.0;
    wi = reflect(-wo, normal);

    return MAT_IDEAL_REFLECTION_REFLECTANCE;
}

vec3 mat_ideal_refraction_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo) {
    return vec3(0.0);
}

vec3 mat_ideal_refraction_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float path_length, out float pdf, inout random_t random) {
    pdf = 1.0;

    if (dot(wo, normal) >= 0.0) {
        wi = refract(-wo, normal, 1.0 / MAT_IDEAL_REFRACTION_IOR);

        if (wi == vec3(0.0)) {
            wi = reflect(-wo, normal);
        }
    } else {
        wi = refract(-wo, -normal, MAT_IDEAL_REFRACTION_IOR);

        if (wi == vec3(0.0)) {
            wi = reflect(-wo, -normal);
        }
    }

    return MAT_IDEAL_REFRACTION_TRANSMITTANCE;
}

vec3 mat_phong_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo) {
    return vec3(0.0); // not used yet
}

vec3 mat_phong_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float path_length, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    float phi = M_2PI * rng.x;
    float theta = acos(pow(rng.y, 1.0 / (MAT_PHONG_EXPONENT + 1.0)));

    wi = rotate(to_spherical(phi, theta), reflect(-wo, normal));

    pdf = 1.0;

    return MAT_PHONG_ALBEDO * (MAT_PHONG_EXPONENT + 2.0) / (MAT_PHONG_EXPONENT + 1.0);
}

vec3 mat_dielectric_eval_brdf(uint inst, vec3 normal, vec3 wi, vec3 wo) {
    return vec3(0.0);
}

vec3 mat_dielectric_sample_brdf(uint inst, vec3 normal, out vec3 wi, vec3 wo, float path_length, out float pdf, inout random_t random) {
    pdf = 1.0;

    float n1, n2, cosI = dot(wo, normal);
    vec3 extinction;

    if (cosI > 0.0) {
        n1 = MAT_DIELECTRIC_EXTERNAL_REFRACTIVE_INDEX;
        n2 = MAT_DIELECTRIC_INTERNAL_REFRACTIVE_INDEX;
        extinction = exp(-MAT_DIELECTRIC_EXTERNAL_EXTINCTION_COEFFICIENT * M_2PI * path_length / vec3(425e-9, 525e-9, 700e-9));
    } else {
        n1 = MAT_DIELECTRIC_INTERNAL_REFRACTIVE_INDEX;
        n2 = MAT_DIELECTRIC_EXTERNAL_REFRACTIVE_INDEX;
        extinction = exp(-MAT_DIELECTRIC_INTERNAL_EXTINCTION_COEFFICIENT * M_2PI * path_length / vec3(425e-9, 525e-9, 700e-9));

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

        if (rand_uniform_vec2(random).x < r) {
            wi = reflect(-wo, normal);
        } else {
            wi = (eta * cosI - cosT) * normal - eta * wo;

            // Account for change in beam area and wave velocity; the change in wave
            // velocity is only important if a light source exists inside the medium
            // as otherwise the factor is cancelled out as the ray exits the medium.

            extinction *= cosT / (cosI * eta);
        }
    } else {
        wi = reflect(-wo, normal);
    }

    return extinction;
}

// Dispatch functions

vec3 mat_eval_brdf(uint material, uint inst, vec3 normal, vec3 wi, vec3 wo) {
    switch (material) {
        case 0U:
            return mat_lambertian_eval_brdf(inst, normal, wi, wo);
        case 1U:
            return mat_ideal_reflection_eval_brdf(inst, normal, wi, wo);
        case 2U:
            return mat_phong_eval_brdf(inst, normal, wi, wo);
        case 3U:
            return mat_ideal_refraction_eval_brdf(inst, normal, wi, wo);
        case 4U:
            return mat_dielectric_eval_brdf(inst, normal, wi, wo);
        default:
            return vec3(0.0);
    }
}

vec3 mat_sample_brdf(uint material, uint inst, vec3 normal, out vec3 wi, vec3 wo, float path_length, out float pdf, inout random_t random) {
    switch (material) {
        case 0U:
            return mat_lambertian_sample_brdf(inst, normal, wi, wo, path_length, pdf, random);
        case 1U:
            return mat_ideal_reflection_sample_brdf(inst, normal, wi, wo, path_length, pdf, random);
        case 2U:
            return mat_phong_sample_brdf(inst, normal, wi, wo, path_length, pdf, random);
        case 3U:
            return mat_ideal_refraction_sample_brdf(inst, normal, wi, wo, path_length, pdf, random);
        case 4U:
            return mat_dielectric_sample_brdf(inst, normal, wi, wo, path_length, pdf, random);
        default:
            return vec3(0.0);
    }
}
