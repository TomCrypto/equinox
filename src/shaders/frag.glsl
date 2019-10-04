#include <common.glsl>
#include <random.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>

out vec3 color;

layout (std140) uniform Camera {
    vec4 origin_plane[4];
    vec4 target_plane[4];
    vec4 aperture_settings;
} camera;

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
} globals;

#define FILTER_DELTA (globals.filter_delta)
#define FRAME_RANDOM (globals.frame_state.xy)
#define FRAME_NUMBER (globals.frame_state.z)

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

// Low-discrepancy sequence generator.
//
// Given a fixed, unchanging key, this will produce a low-discrepancy sequence of 2D points
// as a function of frame number, e.g. on the next frame for the same key the next point in
// the sequence will be produced.

vec2 low_discrepancy_2d(uvec2 key) {
    return fract(vec2((key + FRAME_NUMBER) % 8192U) * vec2(0.7548776662, 0.5698402909));
}

// Begin envmap stuff

uniform sampler2D envmap_pix_tex;

uniform sampler2D envmap_marginal_cdf;
uniform sampler2D envmap_conditional_cdfs;

#define ENVMAP_W 4096
#define ENVMAP_H 2048

// TODO: add PDFs later on (see the PBR book for correct values...)

vec3 sample_envmap(vec3 direction) {
    vec2 uv = direction_to_equirectangular(direction, 0.0);

    return texture(envmap_pix_tex, uv).xyz;
}

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

/*

For now hardcode the marginal CDF (we'll put it somewhere like a uniform buffer later on...)

Store the conditional data as [[normalized CDF, actual function value]]

*/

#if 0

// returns (U, V) of the sampled environment map
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

    // pdf /= MARGINAL_INTEGRAL;

    // See V direction for PDF

    float sampled_u = (float(u_offset) + du) / float(textureSize(envmap_conditional_cdfs, 0).x - 1);

    return equirectangular_to_direction(vec2(sampled_u, sampled_v), 0.0);
}

float envmap_cdf_pdf(vec2 uv) {
    ivec2 uv_int = ivec2(uv * vec2(textureSize(envmap_conditional_cdfs, 0).xy - ivec2(1)));

    float cdf_value_at = texelFetch(envmap_conditional_cdfs, uv_int, 0).y;

    int w = textureSize(envmap_conditional_cdfs, 0).x - 1;
    int h = textureSize(envmap_conditional_cdfs, 0).y - 1;

    return cdf_value_at / MARGINAL_INTEGRAL * float(w * h);
}

#endif


// End envmap stuff

// Begin camera stuff

vec2 evaluate_circular_aperture_uv(inout random_t random) {
    vec2 uv = rand_uniform_vec2(random);

    float a = uv.s * M_2PI;

    return sqrt(uv.t) * vec2(cos(a), sin(a));
}

vec2 evaluate_polygon_aperture_uv(inout random_t random) {
    vec2 uv = rand_uniform_vec2(random); // low_discrepancy_2d(pixel_state);

    float corner = floor(uv.s * camera.aperture_settings.y);

    float u = 1.0 - sqrt(uv.s * camera.aperture_settings.y - corner);
    float v = uv.t * (1.0 - u);

    float a = M_PI * camera.aperture_settings.w;

    float rotation = camera.aperture_settings.z + corner * 2.0 * a;

    float c = cos(rotation);
    float s = sin(rotation);

    vec2 p = vec2((u + v) * cos(a), (u - v) * sin(a));
    return vec2(c * p.x - s * p.y, s * p.x + c * p.y);
}

vec2 evaluate_aperture_uv(inout random_t random) {
    switch (int(camera.aperture_settings.x)) {
        case 0: return evaluate_circular_aperture_uv(random);
        case 1: return evaluate_polygon_aperture_uv(random);       
    }

    return vec2(0.0);
}

vec3 bilinear(vec4 p[4], vec2 uv) {
    return mix(mix(p[0].xyz, p[1].xyz, uv.x), mix(p[2].xyz, p[3].xyz, uv.x), uv.y);
}

void evaluate_primary_ray(inout random_t random, out vec3 pos, out vec3 dir) {
    vec2 raster_uv = (gl_FragCoord.xy + FILTER_DELTA) * raster.dimensions.w;
    raster_uv.x -= (raster.dimensions.x * raster.dimensions.w - 1.0) * 0.5;

    vec3 origin = bilinear(camera.origin_plane, evaluate_aperture_uv(random) * 0.5 + 0.5);

    // TODO: this isn't quite right; this generates a flat focal plane but it should be curved
    // (to be equidistant to the lens)
    // maybe just generate this directly in the shader, pass in the camera kind/parameters
    // but it will do for now, we can extend it later when it's needed

    vec3 target = bilinear(camera.target_plane, raster_uv);

    pos = origin;
    dir = normalize(target - origin);
}

// End camera stuff

void main() {
    random_t random = rand_initialize_from_seed(uvec2(gl_FragCoord.xy) + FRAME_RANDOM);

    ray_t ray;
    evaluate_primary_ray(random, ray.org, ray.dir);

    vec3 radiance = vec3(0.0);
    vec3 throughput = vec3(1.0);

    for (uint bounce = 0U; bounce < 100U; ++bounce) {
        traversal_t traversal = traverse_scene(ray);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geometry_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint inst = traversal.hit.y >> 16U;

            float pdf;

            vec3 wo = -ray.dir;
            vec3 wi;

            vec3 estimate = mat_sample_brdf(material, inst, normal, wi, wo, pdf, random);

            throughput *= estimate / pdf;

            ray.dir = wi;
        } else {
            // we've hit the environment map. We need to sample the environment map...

            radiance += throughput * sample_envmap(ray.dir) * 1.0;

            break;
        }

        // russian roulette

        vec2 rng = rand_uniform_vec2(random);
        float p = min(1.0, max(throughput.x, max(throughput.y, throughput.z)));

        if (rng.x < p) {
            throughput /= p;
        } else {
            break;
        }
    }

    color = radiance;
}
