#include <common.glsl>
#include <random.glsl>

uniform sampler2D envmap_pix_tex;

uniform sampler2D envmap_marginal_cdf;
uniform sampler2D envmap_conditional_cdfs;

float inverse_transform(sampler2D texture, int y, float u, out int index) {
    int l = 0, r = textureSize(texture, 0).x - 1;
    float this_cdf, next_cdf;

    while (l < r) {
        int m = (l + r) / 2;

        float cdf = texelFetch(texture, ivec2(m, y), 0).x;

        if (cdf > u) {
            r = m;
        } else {
            l = m + 1;
            this_cdf = cdf;
        }
    }

    next_cdf = texelFetch(texture, ivec2(l, y), 0).x;

    index = l - 1;

    return (float(index) + (u - this_cdf) / (next_cdf - this_cdf)) / float(textureSize(texture, 0).x - 1);
}

// returns (wi, pdf) for the environment map as well as the light contribution from that direction
// the returned light contribution is PREDIVIDED by the PDF

vec3 env_sample_light_image(out vec3 wi, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    int index;

    float sampled_v = inverse_transform(envmap_marginal_cdf, 0, rng.x, index);
    float sampled_u = inverse_transform(envmap_conditional_cdfs, index, rng.y, index);

    wi = equirectangular_to_direction(vec2(sampled_u, sampled_v), 0.0);

    vec4 value = texture(envmap_pix_tex, direction_to_equirectangular(wi, 0.0));

    pdf = value.w;

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
