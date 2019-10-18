#include <common.glsl>
#include <random.glsl>

uniform sampler2D envmap_pix_tex;

uniform sampler2D envmap_marginal_cdf;
uniform sampler2D envmap_conditional_cdfs;

float inverse_transform(sampler2D texture, int y, float u, out int index) {
    int l = 0, r = textureSize(texture, 0).x - 1;
    vec2 value;

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

    index = l - 1;

    return (float(index)) / float(textureSize(texture, 0).x - 1) + (u - value.x) / value.y;
}

int find_interval(sampler2D texture, int y, float u, out vec2 value) {
    // find the largest index x such that texture[y][x] <= u

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
    int index;

    float sampled_v = inverse_transform(envmap_marginal_cdf, 0, u, index);
    float sampled_u = inverse_transform(envmap_conditional_cdfs, index, v, index);

    return equirectangular_to_direction(vec2(sampled_u, sampled_v), 0.0);
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
