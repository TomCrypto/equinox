#include <common.glsl>
#include <random.glsl>

uniform sampler2D envmap_texture;
uniform sampler2D envmap_marg_cdf;
uniform sampler2D envmap_cond_cdf;

float inverse_transform(sampler2D texture, int y, float u, int size, out int index) {
    int low = 0, high = size;
    float this_cdf, next_cdf;

    // TODO: is the while(true) formula here faster?

    while (low < high) {
        int mid = (low + high) / 2;

        float Fx = texelFetch(texture, ivec2(mid, y), 0).x;

        if (Fx > u) {
            high = mid;
        } else {
            low = mid + 1;
            this_cdf = Fx;
        }
    }

    next_cdf = texelFetch(texture, ivec2(low, y), 0).x;

    index = low - 1;

    return (float(index) + (u - this_cdf) / (next_cdf - this_cdf)) / float(size);
}

// returns (wi, pdf) for the environment map as well as the light contribution from that direction
// the returned light contribution is PREDIVIDED by the PDF

vec3 env_sample_light_image(out vec3 wi, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    int index;

    float sampled_v = inverse_transform(envmap_marg_cdf,     0, rng.x, ENVMAP_ROWS, index);
    float sampled_u = inverse_transform(envmap_cond_cdf, index, rng.y, ENVMAP_COLS, index);

    wi = equirectangular_to_direction(vec2(sampled_u, sampled_v), ENVMAP_ROTATION);

    vec4 value = texture(envmap_texture, vec2(sampled_u, sampled_v));

    float sin_theta = sin(sampled_v * M_PI);

    if (sin_theta == 0.0) {
        pdf = 0.0;
    } else {
        pdf = value.w / sin_theta;
    }

    return value.rgb * sin_theta / value.w;
}

vec3 env_eval_light_image(vec3 wi, out float pdf) {
    vec4 value = texture(envmap_texture, direction_to_equirectangular(wi, ENVMAP_ROTATION));

    float sin_theta = sin(direction_to_equirectangular(wi, ENVMAP_ROTATION).y * M_PI);

    if (sin_theta == 0.0 || isnan(sin_theta)) {
        pdf = 0.0;
    } else {
        pdf = value.w / sin_theta;
    }

    return value.rgb;
}

vec3 env_sample_light_solid(out vec3 wi, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    rng.x = 2.0 * rng.x - 1.0;

    float r = sqrt(1.0 - rng.x * rng.x);
    float phi = M_2PI * rng.y;

    wi = vec3(cos(phi) * r, rng.x, sin(phi) * r);

    pdf = 1.0 / (2.0 * M_PI * M_PI);

    return vec3(1.0) * 2.0 * M_PI * M_PI;
}

vec3 env_eval_light_solid(vec3 wi, out float pdf) {
    pdf = 1.0 / (2.0 * M_PI * M_PI);

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
