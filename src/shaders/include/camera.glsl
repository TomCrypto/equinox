#include <common.glsl>

#include <quasi.glsl>

layout (std140) uniform Camera {
    vec4 origin_plane[4];
    vec4 target_plane[4];
    vec4 aperture_settings;
} camera;

vec2 evaluate_circular_aperture_uv(inout quasi_t quasi) {
    float u1 = quasi_sample(quasi);
    float u2 = quasi_sample(quasi);

    float a = u1 * M_2PI;

    return sqrt(u2) * vec2(cos(a), sin(a));
}

vec2 evaluate_polygon_aperture_uv(inout quasi_t quasi) {
    float u1 = quasi_sample(quasi);
    float u2 = quasi_sample(quasi);

    float corner = floor(u1 * camera.aperture_settings.y);

    float u = 1.0 - sqrt(u1 * camera.aperture_settings.y - corner);
    float v = u2 * (1.0 - u);

    float a = M_PI * camera.aperture_settings.w;

    float rotation = camera.aperture_settings.z + corner * 2.0 * a;

    float c = cos(rotation);
    float s = sin(rotation);

    vec2 p = vec2((u + v) * cos(a), (u - v) * sin(a));
    return vec2(c * p.x - s * p.y, s * p.x + c * p.y);
}

vec2 evaluate_aperture_uv(inout quasi_t quasi) {
    switch (int(camera.aperture_settings.x)) {
        case 0: return evaluate_circular_aperture_uv(quasi);
        case 1: return evaluate_polygon_aperture_uv(quasi);       
    }

    return vec2(0.0);
}

vec3 bilinear(vec4 p[4], vec2 uv) {
    return mix(mix(p[0].xyz, p[1].xyz, uv.x), mix(p[2].xyz, p[3].xyz, uv.x), uv.y);
}

void evaluate_primary_ray(out vec3 pos, out vec3 dir, inout quasi_t quasi) {
    vec2 raster_uv = (gl_FragCoord.xy + integrator.filter_offset) * raster.dimensions.w;
    raster_uv.x -= (raster.dimensions.x * raster.dimensions.w - 1.0) * 0.5;

    vec3 origin = bilinear(camera.origin_plane, evaluate_aperture_uv(quasi) * 0.5 + 0.5);

    // TODO: this isn't quite right; this generates a flat focal plane but it should be curved
    // (to be equidistant to the lens)
    // maybe just generate this directly in the shader, pass in the camera kind/parameters
    // but it will do for now, we can extend it later when it's needed

    vec3 target = bilinear(camera.target_plane, raster_uv);

    pos = origin;
    dir = normalize(target - origin);
}
