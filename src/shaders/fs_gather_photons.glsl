#include <common.glsl>
#include <random.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>
#include <integrator.glsl>

uniform sampler2D li_range_tex;
uniform sampler2D photon_table_major;
uniform sampler2D photon_table_minor;

layout(location = 0) out vec4 ld_count_output;
layout(location = 1) out vec4 li_count_output;

layout (std140) uniform Camera {
    vec4 origin_plane[4];
    vec4 target_plane[4];
    vec4 aperture_settings;
} camera;

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

vec3 get_photon(vec3 cell_pos, vec3 point, float radius_squared, uint material, uint inst, vec3 normal, vec3 wo, inout float count) {
    if (!sphere_in_cell_broadphase(radius_squared, point, cell_pos)) {
        return vec3(0.0);
    }

    ivec2 coords = hash_entry_for_cell(cell_pos);

    vec3 result = vec3(0.0);

    for (uint y = 0U; y < integrator.hash_cell_rows; ++y) {
        for (uint x = 0U; x < integrator.hash_cell_cols; ++x) {
            vec4 major_data = texelFetch(photon_table_major, coords + ivec2(x, y), 0);

            vec3 photon_position = (cell_pos + major_data.xyz) * integrator.cell_size;

            if (dot(point - photon_position, point - photon_position) > radius_squared) {
                continue;
            }

            vec4 minor_data = texelFetch(photon_table_minor, coords + ivec2(x, y), 0);

            vec3 photon_throughput = minor_data.xyz;

            float sgn = any(lessThan(photon_throughput, vec3(0.0))) ? -1.0 : 1.0;

            vec3 photon_direction = vec3(major_data.w, sqrt(max(0.0, 1.0 - major_data.w * major_data.w - minor_data.w * minor_data.w)) * sgn, minor_data.w);

            float pdf;
            count += 1.0;
            result += abs(photon_throughput) * mat_eval_brdf(material, inst, normal, -photon_direction, wo, pdf);
        }
    }

    return result;
}

vec3 gather_photons(float radius_squared, vec3 position, vec3 direction, vec3 normal, uint material, uint inst, out float count) {
    if (radius_squared == 0.0) {
        return vec3(0.0);
    }

    vec3 cell_pos = floor(position / integrator.cell_size);
    vec3 in_pos = fract(position / integrator.cell_size);

    vec3 dir = sign(in_pos - vec3(0.5));

    vec3 accumulation = vec3(0.0);

    accumulation += get_photon(cell_pos + dir * vec3(0.0, 0.0, 0.0), position, radius_squared, material, inst, normal, -direction, count);
    accumulation += get_photon(cell_pos + dir * vec3(0.0, 0.0, 1.0), position, radius_squared, material, inst, normal, -direction, count);
    accumulation += get_photon(cell_pos + dir * vec3(0.0, 1.0, 0.0), position, radius_squared, material, inst, normal, -direction, count);
    accumulation += get_photon(cell_pos + dir * vec3(0.0, 1.0, 1.0), position, radius_squared, material, inst, normal, -direction, count);
    accumulation += get_photon(cell_pos + dir * vec3(1.0, 0.0, 0.0), position, radius_squared, material, inst, normal, -direction, count);
    accumulation += get_photon(cell_pos + dir * vec3(1.0, 0.0, 1.0), position, radius_squared, material, inst, normal, -direction, count);
    accumulation += get_photon(cell_pos + dir * vec3(1.0, 1.0, 0.0), position, radius_squared, material, inst, normal, -direction, count);
    accumulation += get_photon(cell_pos + dir * vec3(1.0, 1.0, 1.0), position, radius_squared, material, inst, normal, -direction, count);

    return accumulation;
}

// Begin camera stuff

vec2 evaluate_circular_aperture_uv(inout random_t random) {
    vec2 uv = rand_uniform_vec2(random);

    float a = uv.s * M_2PI;

    return sqrt(uv.t) * vec2(cos(a), sin(a));
}

vec2 evaluate_polygon_aperture_uv(inout random_t random) {
    vec2 uv = rand_uniform_vec2(random);

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
    vec2 raster_uv = (gl_FragCoord.xy + integrator.filter_offset) * raster.dimensions.w;
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

vec3 sample_direct_lighting(ray_t ray, vec3 normal, uint material, uint mat_inst, random_t random) {
    float lightPdf, scatterPdf;

    vec3 wi;
    vec3 wo = -ray.dir;

    vec3 Li = env_sample_light(wi, lightPdf, random);
    vec3 radiance;

    if (lightPdf != 0.0) {
        vec3 f = mat_eval_brdf(material, mat_inst, normal, wi, wo, scatterPdf) * abs(dot(wi, normal));

        if (scatterPdf != 0.0 && !is_ray_occluded(make_ray(ray.org, wi, normal), 1.0 / 0.0)) {
            radiance += f * Li * power_heuristic(lightPdf, scatterPdf);
        }
    }

    vec3 f = mat_sample_brdf(material, mat_inst, normal, wi, wo, scatterPdf, random);

    if (scatterPdf != 0.0 && !is_ray_occluded(make_ray(ray.org, wi, normal), 1.0 / 0.0)) {
        Li = env_eval_light(wi, lightPdf);

        if (lightPdf != 0.0) {
            radiance += f * Li * power_heuristic(scatterPdf, lightPdf);
        }
    }

    return radiance;
}

// End camera stuff

void main() {
    vec3 radiance = vec3(0.0);

    random_t random = rand_initialize_from_seed(uvec2(gl_FragCoord.xy) + integrator.rng);

    ray_t ray;
    evaluate_primary_ray(random, ray.org, ray.dir);

    vec3 throughput = vec3(1.0);
    uint traversal_start = 0U;
    uint flags;
    float unused_pdf;
    bool explicit_light = false;

    for (uint bounce = 0U; bounce < 8U; ++bounce) {
        traversal_t traversal = traverse_scene(ray, traversal_start);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;

            bool is_receiver = (material & 0x8000U) != 0U;
            material &= ~0x8000U;

            // TODO: add back a direct light sampling flag? ...
            if (mat_is_not_specular(material)) {
                radiance += throughput * sample_direct_lighting(ray, normal, material, mat_inst, random);
                explicit_light = true;
            } else {
                explicit_light = false;
            }

            if (is_receiver) {
                // we found our diffuse surface, record the hit...
                float radius_squared = texelFetch(li_range_tex, ivec2(gl_FragCoord - 0.5), 0).w;
                radius_squared = min(radius_squared, pow(integrator.cell_size * 0.5, 2.0));

                float count = 0.0;

                vec3 indirect_radiance = throughput * gather_photons(radius_squared, ray.org, ray.dir, normal, material, mat_inst, count);

                ld_count_output = vec4(radiance, count);
                li_count_output = vec4(indirect_radiance, count);

                return;
            } else {
                // NOT DIFFUSE: just keep tracing as usual...
                vec3 beta;
                ray = mat_interact(material, mat_inst, normal, -ray.dir, ray.org, traversal.range.y, beta, flags, random);
                throughput *= beta;

                if ((flags & RAY_FLAG_EXTINCT) != 0U) {
                    break; // no need to trace further
                }

                if (((~flags) & (RAY_FLAG_OUTSIDE | RAY_FLAG_TRANSMIT)) == 0U) {
                    traversal_start = traversal.hit.z;
                } else {
                    traversal_start = 0U;
                }
            }            
        } else {
            if (!explicit_light) {
                radiance += throughput * env_eval_light(ray.dir, unused_pdf);
            }

            break;
        }

        /*if (bounce <= 2U) {
            continue;
        }

        // russian roulette

        vec2 rng = rand_uniform_vec2(random);
        float p = min(1.0, max(throughput.x, max(throughput.y, throughput.z)));

        if (rng.x < p) {
            throughput /= p;
        } else {
            break;
        }*/
    }

    ld_count_output = vec4(radiance, 0.0);
    li_count_output = vec4(0.0);
}
