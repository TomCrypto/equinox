#include <common.glsl>
#include <random.glsl>

uniform sampler2D envmap_pix_tex;

uniform sampler2D envmap_marginal_cdf;
uniform sampler2D envmap_conditional_cdfs;

/*

Rework this:

 * store CDF + delta to next CDF in the texture
 * return the two from find_interval
 * this avoids 4 extra unnecessary texelFetch

If l < r, and r = (l + r) / 2, can l >= r??


l = 3
r = 4

(3 + 4) / 2 = 3

*/

int find_interval(sampler2D texture, int y, float u, out vec2 value) {
    // find the largest index x such that texture[y][x] <= u

    // value = texelFetch(texture, ivec2(0, y), 0).xy;

    int l = 0, r = textureSize(texture, 0).x - 1;

    while (l < r) {
        int m = (l + r) / 2;

        vec2 data = texelFetch(texture, ivec2(m, y), 0).xy;

        if (data.x > u) {
            r = m;
        } else {
            l = m + 1;
            value = data;
        }
    }

    return l - 1;
}

vec3 importance_sample_envmap(float u, float v) {
    // V DIRECTION (marginal CDF)

    vec2 value;

    int v_offset = find_interval(envmap_marginal_cdf, 0, u, value);

    float v_cdf_at_offset = value.x; // texelFetch(envmap_marginal_cdf, ivec2(v_offset, 0), 0).x;
    float v_cdf_at_offset_next = value.y; // texelFetch(envmap_marginal_cdf, ivec2(v_offset + 1, 0), 0).x;

    // linearly interpolate between u_offset and u_offset + 1 based on position of u between cdf_at_offset and u_cdf_at_offset_next
    float dv = (u - v_cdf_at_offset) / value.y; // (v_cdf_at_offset_next - v_cdf_at_offset);

    float sampled_v = (float(v_offset) + dv) / float(textureSize(envmap_marginal_cdf, 0).x - 1);

    // U DIRECTION (conditional CDF)

    int u_offset = find_interval(envmap_conditional_cdfs, v_offset, v, value);

    float u_cdf_at_offset = value.x; // texelFetch(envmap_conditional_cdfs, ivec2(u_offset, v_offset), 0).x;
    float u_cdf_at_offset_next = value.y; // texelFetch(envmap_conditional_cdfs, ivec2(u_offset + 1, v_offset), 0).x;

    float du = (v - u_cdf_at_offset) / value.y; // (u_cdf_at_offset_next - u_cdf_at_offset);

    float sampled_u = (float(u_offset) + du) / float(textureSize(envmap_conditional_cdfs, 0).x - 1);

    vec3 direction = equirectangular_to_direction(vec2(sampled_u, sampled_v), 0.0);

    /*if (isinf(du) || isinf(dv)) {
        return vec3(0.0);
    }*/

    return direction;
}

// returns (wi, pdf) for the environment map as well as the light contribution from that direction
// the returned light contribution is PREDIVIDED by the PDF

vec3 env_sample_light_image(out vec3 wi, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    wi = importance_sample_envmap(rng.x, rng.y);

    vec4 value = texture(envmap_pix_tex, direction_to_equirectangular(wi, 0.0));

    if (wi != vec3(0.0)) {
        pdf = value.w;
    } else {
        pdf = 0.0;
    }

    return value.rgb / pdf;
}

vec3 env_eval_light_image(vec3 wi, out float pdf) {
    vec4 value = texture(envmap_pix_tex, direction_to_equirectangular(wi, 0.0));

    pdf = value.w;

    return value.rgb;
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
