layout (std140) uniform Geometry {
    vec4 data[GEOMETRY_DATA_COUNT];
} geometry_buffer;

float geometry_distance(uint geometry, uint inst, vec3 p);
vec3 geometry_normal(uint geometry, uint inst, vec3 p);

#include <geometry-user.glsl>

bool ray_sdf(ray_t ray, uint geometry, uint instance, inout vec2 range) {
    while (range.x <= range.y) {
        // need to take the absolute value here in case we're on the inside of a distance field
        // I'm not sure if this is always valid?
        float dist = abs(geometry_distance(geometry, instance, ray.org + range.x * ray.dir));

        if (dist < PREC * 0.1) {
            return true;
        }

        range.x += dist;
    }

    return false;
}
