#include <common.glsl>
#include <random.glsl>

#include <instance.glsl>

uniform sampler2D envmap_pix_tex;

uniform sampler2D envmap_marginal_cdf;
uniform sampler2D envmap_conditional_cdfs;

uint find_interval(sampler2D texture, int y, float u) {
    uint first = 0U;
    uint size = uint(textureSize(texture, 0).x);
    uint len = size;

    while (len > 0U) {
        uint _half = len >> 1U;
        uint middle = first + _half;

        float value = texelFetch(texture, ivec2(int(middle), y), 0).x;

        if (value <= u) {
            first = middle + 1U;
            len -= _half + 1U;
        } else {
            len = _half;
        }
    }

    return clamp(first - 1U, 0U, size - 2U);
}

vec3 importance_sample_envmap(float u, float v, out float pdf) {
    // V DIRECTION (marginal CDF)

    uint v_offset = find_interval(envmap_marginal_cdf, 0, u);

    float v_cdf_at_offset = texelFetch(envmap_marginal_cdf, ivec2(int(v_offset), 0), 0).x;
    float v_cdf_at_offset_next = texelFetch(envmap_marginal_cdf, ivec2(int(v_offset) + 1, 0), 0).x;

    // linearly interpolate between u_offset and u_offset + 1 based on position of u between cdf_at_offset and u_cdf_at_offset_next
    float dv = (u - v_cdf_at_offset) / (v_cdf_at_offset_next - v_cdf_at_offset);

    pdf = texelFetch(envmap_marginal_cdf, ivec2(int(v_offset), 0), 0).y;

    /*float dv = v - v_cdf_at_offset;

    if (v_cdf_at_offset_next != v_cdf_at_offset) {
        dv /= (v_cdf_at_offset_next - v_cdf_at_offset);
    }*/

    // PDF is func[offset] / funcInt which (IIUC) is just (cdf_at_offset_next - cdf_at_offset)

    float sampled_v = (float(v_offset) + dv) / float(textureSize(envmap_marginal_cdf, 0).x - 1);

    // U DIRECTION (conditional CDF)

    uint u_offset = find_interval(envmap_conditional_cdfs, int(v_offset), v);

    float u_cdf_at_offset = texelFetch(envmap_conditional_cdfs, ivec2(int(u_offset), v_offset), 0).x;
    float u_cdf_at_offset_next = texelFetch(envmap_conditional_cdfs, ivec2(int(u_offset) + 1, v_offset), 0).x;

    /*float du = v - u_cdf_at_offset;

    if (u_cdf_at_offset_next != u_cdf_at_offset) {
        du /= (u_cdf_at_offset_next - u_cdf_at_offset);
    }*/

    float du = (v - u_cdf_at_offset) / (u_cdf_at_offset_next - u_cdf_at_offset);

    pdf *= texelFetch(envmap_conditional_cdfs, ivec2(int(u_offset), v_offset), 0).y;

    pdf *= 4096.0 * 2048.0 / M_2PI; // TODO: sin(theta) factor needed here!

    // pdf /= MARGINAL_INTEGRAL;

    // See V direction for PDF

    float sampled_u = (float(u_offset) + du) / float(textureSize(envmap_conditional_cdfs, 0).x - 1);

    vec3 direction = equirectangular_to_direction(vec2(sampled_u, sampled_v), 0.0);

    // TODO: this should never actually occur (it implies we selected a value with PDF 0)

    if (isinf(du) || isinf(dv)) {
        pdf = 0.0;
    }

    return direction;
}

// returns (wi, pdf) for the environment map as well as the light contribution from that direction
// the returned light contribution is PREDIVIDED by the PDF

vec3 env_sample_light_image(out vec3 wi, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    wi = importance_sample_envmap(rng.x, rng.y, pdf);

    return texture(envmap_pix_tex, direction_to_equirectangular(wi, 0.0)).xyz / pdf;
}

vec3 env_eval_light_image(vec3 wi, out float pdf) {
    vec2 uv = direction_to_equirectangular(wi, 0.0);

    uv.x = fract(uv.x + 1.0);

    int py = int((uv.y + 0.5) * float(textureSize(envmap_marginal_cdf, 0).x));
    int px = int((uv.x + 0.5) * float(textureSize(envmap_conditional_cdfs, 0).x - 1));

    pdf = texelFetch(envmap_marginal_cdf, ivec2(py, 0), 0).y;
    pdf *= texelFetch(envmap_conditional_cdfs, ivec2(px, py), 0).y;
    pdf *= 4096.0 * 2048.0 / M_2PI; // TODO: sin(theta) factor needed here!

    return texture(envmap_pix_tex, uv).xyz;
}

vec3 env_sample_light_solid(out vec3 wi, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    rng.x = 2.0 * rng.x - 1.0;

    float r = sqrt(1.0 - rng.x * rng.x);
    float phi = M_2PI * rng.y;

    wi = vec3(cos(phi) * r, rng.x, sin(phi) * r);

    pdf = 1.0 / M_4PI;

    return vec3(1.0) * M_4PI;
}

vec3 env_eval_light_solid(vec3 wi, out float pdf) {
    pdf = 1.0 / M_4PI;

    return vec3(1.0);
}

vec3 env_sample_light(out vec3 wi, out float pdf, inout random_t random) {
#if HAS_ENVMAP
    return env_sample_light_image(wi, pdf, random);
#else
    return env_sample_light_solid(wi, pdf, random);
#endif
}

vec3 env_eval_light(vec3 wi, out float pdf) {
#if HAS_ENVMAP
    return env_eval_light_image(wi, pdf);
#else
    return env_eval_light_solid(wi, pdf);
#endif
}
