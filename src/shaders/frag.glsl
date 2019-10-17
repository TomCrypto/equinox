#include <common.glsl>
#include <random.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>

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

#if 0

vec3 sample_envmap(vec3 direction) {
#if HAS_ENVMAP
    vec2 uv = direction_to_equirectangular(direction, 0.0);

    return texture(envmap_pix_tex, uv).xyz;
#else
    return vec3(1.0);
#endif
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

float envmap_pdf(vec3 point, vec3 normal, vec3 direction) {
#if HAS_ENVMAP
    if (dot(direction, normal) <= 0.0 || is_ray_occluded(ray_t(point + normal * PREC * sign(dot(direction, normal)), direction), 1.0 / 0.0)) {
        return 0.0;
    }

    vec2 uv = direction_to_equirectangular(direction, 0.0);
    uv.x = fract(uv.x + 1.0);

    int py = int((uv.y + 0.5) * float(textureSize(envmap_marginal_cdf, 0).x));
    int px = int((uv.x + 0.5) * float(textureSize(envmap_conditional_cdfs, 0).x - 1));

    float pdf = texelFetch(envmap_marginal_cdf, ivec2(py, 0), 0).y;
    pdf *= texelFetch(envmap_conditional_cdfs, ivec2(px, py), 0).y;
    pdf *= 4096.0 * 2048.0 / M_2PI; // TODO: sin(theta) factor needed here!

    return pdf;
#else
    return 1.0 / M_2PI;
#endif
}

// returns (U, V) of the sampled environment map
vec3 importance_sample_envmap(vec3 point, vec3 normal, float u, float v, out float pdf) {
#if HAS_ENVMAP
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
        direction = -normal;
    }

    if ((dot(direction, normal) <= 0.0) || is_ray_occluded(ray_t(point + normal * PREC * sign(dot(direction, normal)), direction), 1.0 / 0.0)) {
        pdf = 0.0;
    }

    return direction;
#else
    // random uniform direction in hemisphere...
    float r = sqrt(1.0 - u * u);
    float phi = M_2PI * v;

    pdf = 1.0 / M_2PI;

    return rotate(vec3(cos(phi) * r, u, sin(phi) * r), normal);
#endif
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

float PowerHeuristic(float fPdf, float gPdf) {
    float f = fPdf;
    float g = gPdf;

    return (f * f) / (f * f + g * g);
}

// TODO: factor in path_length later somehow
// TODO: optimize the power heuristic to avoid unnecessary divisions that cancel out

// call this to get an estimate of the direct lighting hitting a point
// don't call this for specular surfaces
// and if this is called, do not consider the environment map on the next bounce
// TODO: do we need to consider next bounce lighting if the PDF was zero? (not sure)
vec3 estimate_direct_lighting(vec3 point, uint material, uint inst, vec3 wo, vec3 normal, inout random_t random) {
    // sample lighting with multiple importance sampling

    float lightPdf, scatteringPdf;
    vec3 f;
    vec3 directLighting;

    vec3 light_direction;
    vec3 Li = env_sample_light(light_direction, lightPdf, random);

    // TODO: special logic to allow transmissive in some materials?

    float cosTheta = dot(light_direction, normal);

    if (cosTheta <= 0.0 || is_ray_occluded(make_ray(point, light_direction, normal), 1.0 / 0.0)) {
        lightPdf = 0.0;
    }

    // Make sure the pdf isn't zero and the radiance isn't black
    if (lightPdf != 0.0 && cosTheta > 0.0) {
        // Calculate the brdf value
        f = mat_eval_brdf(material, inst, normal, light_direction, wo, scatteringPdf) * cosTheta;

        if (scatteringPdf != 0.0) {
            float weight = PowerHeuristic(lightPdf, scatteringPdf);
            directLighting += f * Li * weight;
        }
    }

    // Sample brdf with multiple importance sampling
    vec3 wi;
    f = mat_sample_brdf(material, inst, normal, wi, wo, 0.0, scatteringPdf, random);

    if (scatteringPdf != 0.0) {
        vec3 Li = env_eval_light(wi, lightPdf);

        if (!is_ray_occluded(make_ray(point, wi, normal), 1.0 / 0.0)) {
            float weight = PowerHeuristic(scatteringPdf, lightPdf);
            directLighting += f * Li * weight / scatteringPdf;
        }
    }

    return directLighting;
}

void main() {
    random_t random = rand_initialize_from_seed(uvec2(gl_FragCoord.xy) + FRAME_RANDOM);

    ray_t ray;
    evaluate_primary_ray(random, ray.org, ray.dir);

    vec3 radiance = vec3(0.0);
    vec3 strength = vec3(1.0);
    bool last_is_specular = true;

    for (uint bounce = 0U; bounce < 100U; ++bounce) {
        traversal_t traversal = traverse_scene(ray);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint inst = traversal.hit.y >> 16U;

            /*

            ray.dir = mat_interact(material, instance, ray.org, -ray.dir, traversal.range.y, normal, random, strength, sampled_light);
            ray.org += normal * PREC * sign(dot(ray.dir, normal));

            // on environment hit, if !sampled_light then add envmap contribution

            */

            vec3 wo = -ray.dir;

            bool specular = mat_is_specular(material);

            if (!specular) {
                vec3 direct = estimate_direct_lighting(ray.org, material, inst, wo, normal, random);

                radiance += strength * direct;
            }

            last_is_specular = specular;

            vec3 wi;
            float brdf_pdf;

            vec3 brdf_path_strength = mat_sample_brdf(material, inst, normal, wi, wo, traversal.range.y, brdf_pdf, random);

            strength *= brdf_path_strength / brdf_pdf;

            ray = make_ray(ray.org, wi, normal);
        } else {
            // we've hit the environment map. We need to sample the environment map...

            float lightPdf;

            if (last_is_specular) {
                radiance += strength * env_eval_light(ray.dir, lightPdf);
            }

            break;
        }

        if (bounce <= 1U) {
            continue;
        }

        // russian roulette

        vec2 rng = rand_uniform_vec2(random);
        float p = min(1.0, max(strength.x, max(strength.y, strength.z)));

        if (rng.x < p) {
            strength /= p;
        } else {
            break;
        }
    }

    color = radiance;
}
