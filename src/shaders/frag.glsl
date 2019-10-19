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

    if (lightPdf != 0.0 && (cosTheta <= 0.0 || is_ray_occluded(make_ray(point, light_direction, normal), 1.0 / 0.0))) {
        lightPdf = 0.0;
    }

    // Make sure the pdf isn't zero and the radiance isn't black
    if (lightPdf != 0.0) {
        // Calculate the brdf value
        f = mat_eval_brdf(material, inst, normal, light_direction, wo, scatteringPdf) * abs(cosTheta);

        if (scatteringPdf != 0.0) {
            float weight = PowerHeuristic(lightPdf, scatteringPdf);
            directLighting += f * Li * weight;
        }
    }

    // Sample brdf with multiple importance sampling
    vec3 wi;
    f = mat_sample_brdf(material, inst, normal, wi, wo, 0.0, scatteringPdf, random);

    if (scatteringPdf != 0.0) {
        if (!is_ray_occluded(make_ray(point, wi, normal), 1.0 / 0.0)) {
            vec3 Li = env_eval_light(wi, lightPdf);

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
    bool caustic_path = false;
    uint index = 0U;

    for (uint bounce = 0U; bounce < 100U; ++bounce) {
        traversal_t traversal = traverse_scene(ray, index);

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

            // if the last hit was not specular, and we are specular, this is a caustic
            if (!last_is_specular && specular) {
                caustic_path = true;
            }

            if (!specular) {
                vec3 direct = estimate_direct_lighting(ray.org, material, inst, wo, normal, random);

                radiance += strength * direct;
            }

            last_is_specular = specular;

            vec3 wi;
            float brdf_pdf;

            vec3 brdf_path_strength = mat_sample_brdf(material, inst, normal, wi, wo, traversal.range.y, brdf_pdf, random);

            // TODO: better way to represent this?

            if (brdf_pdf == 0.0) {
                break;
            }

            strength *= brdf_path_strength / brdf_pdf;

            bool outside = dot(ray.dir, normal) <= 0.0;

            ray = make_ray(ray.org, wi, normal);

            // normal always points OUTWARDS, so we are only refracting if:
            //  - we were going towards the normal before
            //  - we are going the same direction after

            if (outside && (dot(ray.dir, normal) <= 0.0)) {
                // we are refracting
                index = traversal.hit.z;
            } else {
                index = 0U; // restart from the root
            }
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

    //if (caustic_path) {
        color = radiance;
    /*} else {
        color = vec3(0.0);
    }*/
}
