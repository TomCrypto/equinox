layout (std140) uniform Geometry {
    vec4 data[64];
} geometry_buffer;

float geometry_distance(uint geometry, uint inst, vec3 p);
vec3 geometry_normal(uint geometry, uint inst, vec3 p);

#include <geometry-user.glsl>

bool ray_sdf(ray_t ray, uint geometry, uint instance, inout vec2 range) {
    // TODO: possibly dynamically adjust precision based on initial distance?
    // this would be neat if it worked honestly

    // float prec = range.y / 1000.0;

    while (range.x <= range.y) {
        // need to take the absolute value here in case we're on the inside of a distance field
        // I'm not sure if this is always valid?
        float dist = abs(geometry_distance(geometry, instance, ray.org + range.x * ray.dir));

        if (dist < PREC) {
            return true;
        }

        range.x += dist;
    }

    return false;
}
