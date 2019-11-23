// requires-define GEOMETRY_DATA_LEN

layout (std140) uniform Geometry {
    vec4 data[GEOMETRY_DATA_LEN];
} geometry_buffer;

bool geo_intersect(uint geometry, uint inst, ray_t ray, inout vec2 range);
vec3 geo_normal(uint geometry, uint inst, vec3 p);

#include <geometry-user.glsl>
