#include <common.glsl>

#include <quasi.glsl>

layout (std140) uniform Camera {
    vec4 aperture_settings;
    mat4 camera_transform;
    vec4 camera_settings;
} camera;

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

vec2 evaluate_circular_aperture_point(float u1, float u2) {
    float a = u1 * M_2PI;

    return sqrt(u2) * vec2(cos(a), sin(a)) * camera.aperture_settings.w;
}

vec2 evaluate_polygon_aperture_point(float u1, float u2) {
    float corner = floor(u1 * camera.aperture_settings.y);

    float u = 1.0 - sqrt(u1 * camera.aperture_settings.y - corner);
    float v = u2 * (1.0 - u);

    float a = M_PI / camera.aperture_settings.y;

    float rotation = camera.aperture_settings.z + corner * 2.0 * a;

    float c = cos(rotation);
    float s = sin(rotation);

    vec2 p = vec2((u + v) * cos(a), (u - v) * sin(a));
    p = vec2(c * p.x - s * p.y, s * p.x + c * p.y);
    return p * camera.aperture_settings.w;
}

vec2 evaluate_aperture_point(inout quasi_t quasi) {
    float u1 = quasi_sample(quasi);
    float u2 = quasi_sample(quasi);

    switch (int(camera.aperture_settings.x)) {
        case 0: return evaluate_circular_aperture_point(u1, u2);
        case 1: return evaluate_polygon_aperture_point(u1, u2);
    }

    return vec2(0.0);
}

ray_t evaluate_camera_ray(vec2 fragment, inout quasi_t quasi) {
    vec2 uv = (fragment + integrator.filter_offset) * raster.dimensions.zw * 2.0 - 1.0;
    uv.x *= raster.dimensions.x * raster.dimensions.w; // maintain camera aspect ratio

    vec3 origin = vec3(evaluate_aperture_point(quasi), 0.0);

    vec3 direction = vec3(uv * camera.camera_settings.x, 1.0);
    float cos_theta_squared = 1.0 / dot(direction, direction);

    float a = camera.camera_settings.z * (1.0 - cos_theta_squared) / cos_theta_squared;
    float c = camera.camera_settings.y;

    vec3 target = direction * 2.0 * c / (1.0 + sqrt(1.0 + 4.0 * a * c));

    origin = (camera.camera_transform * vec4(origin, 1.0)).xyz;
    target = (camera.camera_transform * vec4(target, 1.0)).xyz;

    return ray_t(origin, normalize(target - origin));
}
