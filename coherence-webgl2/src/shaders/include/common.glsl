struct ray_t {
    vec3 org;
    vec3 dir;
};

struct instance_indices_t {
    uvec4 instance_indices;
};

#define ACCEL_ROOT_NODE(indices) (indices.instance_indices.x)
#define TOPOLOGY_OFFSET(indices) (indices.instance_indices.y)
#define GEOMETRY_OFFSET(indices) (indices.instance_indices.z)
#define MATERIAL_OFFSET(indices) (indices.instance_indices.w)

// Maintains closest-hit information during a traversal.
struct traversal_t {
    uvec2 hit; // packed data for the closest SDF hit (geometry/material ID + parameter offsets)
    vec2 range; // min/max of the ray distance
};

traversal_t new_traversal(float near) {
    return traversal_t(uvec2(0xffffffffU), vec2(near, 1.0 / 0.0));
}

bool traversal_has_hit(traversal_t traversal) {
    return traversal.hit.x != 0xffffffffU;
}

bool ray_bbox(vec3 org, vec3 idir, vec3 bmin, vec3 bmax, in traversal_t traversal) {
    vec3 bot = (bmin - org) * idir;
    vec3 top = (bmax - org) * idir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    float near = max(max(tmin.x, tmin.y), tmin.z);
    float far = min(min(tmax.x, tmax.y), tmax.z);

    return (near <= far) && (far > traversal.range.x) && (near < traversal.range.y);
}

// takes in a ray range, and constrains the range to the actual intersection
// returns false if no intersection took place, of course
bool ray_bbox(vec3 org, vec3 idir, vec3 bmin, vec3 bmax, inout vec2 range) {
    vec3 bot = (bmin - org) * idir;
    vec3 top = (bmax - org) * idir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    float near = max(max(tmin.x, tmin.y), tmin.z);
    float far = min(min(tmax.x, tmax.y), tmax.z);

    if (near > far) {
        return false;
    }

    range.x = max(near, range.x);
    range.y = min(far, range.y);

    return range.x <= range.y;
}
