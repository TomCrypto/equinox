#include <common.glsl>
#include <random.glsl>

uniform sampler2D envmap_texture;
uniform sampler2D envmap_marg_cdf;
uniform sampler2D envmap_cond_cdf;

layout (std140) uniform Environment {
    int cols;
    int rows;
    float rotation;
    int has_envmap;
    vec3 tint;
} environment;

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

    index = clamp(low - 1, 0, size - 2);

    float du = u - this_cdf;

    next_cdf = texelFetch(texture, ivec2(index + 1, y), 0).x;

    if (next_cdf - this_cdf > 0.0) {
        du /= (next_cdf - this_cdf);
    }

    return (float(index) + du) / float(size);
}

vec3 env_sample_light_image(out vec3 wi, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    int index;

    float sampled_v = inverse_transform(envmap_marg_cdf,     0, rng.x, environment.rows, index);
    float sampled_u = inverse_transform(envmap_cond_cdf, index, rng.y, environment.cols, index);

    wi = equirectangular_to_direction(vec2(sampled_u, sampled_v), environment.rotation);

    vec4 value = textureLod(envmap_texture, vec2(sampled_u, sampled_v), 0.0);

    float sin_theta = sin(sampled_v * M_PI);

    if (sin_theta == 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = value.w / (sin_theta * 2.0 * M_PI * M_PI * 1e-3);

    return environment.tint * value.rgb * (sin_theta * 2.0 * M_PI * M_PI * 1e-3) / value.w;
}

vec3 env_eval_light_image(vec3 wi, out float pdf) {
    vec4 value = textureLod(envmap_texture, direction_to_equirectangular(wi, environment.rotation), 0.0);

    float sin_theta = sqrt(max(0.0, 1.0 - wi.y * wi.y));

    if (sin_theta == 0.0) {
        return pdf = 0.0, vec3(0.0);
    }

    pdf = value.w / (sin_theta * 2.0 * M_PI * M_PI * 1e-3);

    return environment.tint * value.rgb;
}

vec3 env_sample_light_solid(out vec3 wi, out float pdf, inout random_t random) {
    vec2 rng = rand_uniform_vec2(random);

    rng.x = 2.0 * rng.x - 1.0;

    float r = sqrt(1.0 - rng.x * rng.x);
    float phi = M_2PI * rng.y;

    wi = vec3(cos(phi) * r, rng.x, sin(phi) * r);

    pdf = 1.0 / M_4PI;

    return environment.tint * M_4PI;
}

vec3 env_eval_light_solid(vec3 wi, out float pdf) {
    pdf = 1.0 / M_4PI;

    return environment.tint;
}

vec3 env_sample_light(out vec3 wi, out float pdf, inout random_t random) {
    if (environment.has_envmap == 1) {
        return env_sample_light_image(wi, pdf, random);
    } else {
        return env_sample_light_solid(wi, pdf, random);
    }
}

vec3 env_eval_light(vec3 wi, out float pdf) {
    if (environment.has_envmap == 1) {
        return env_eval_light_image(wi, pdf);
    } else {
        return env_eval_light_solid(wi, pdf);
    }
}
