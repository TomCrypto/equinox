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

traversal_t traversal_prepare(float near) {
    return traversal_t(uvec2(0xffffffffU), vec2(near, 1.0 / 0.0));
}

void traversal_record_hit(inout traversal_t traversal, float distance, uvec2 hit) {
    traversal = traversal_t(hit, vec2(traversal.range.x, distance));
}

bool traversal_has_hit(traversal_t traversal) {
    return traversal.hit.x != 0xffffffffU;
}

// Takes a ray segment and a bounding box and cuts the ray to be fully contained
// inside the bounding box. Returns true if the ray intersects the bounding box.
bool ray_bbox(vec3 org, vec3 idir, inout vec2 range, vec3 bbmin, vec3 bbmax) {
    vec3 bot = (bbmin - org) * idir;
    vec3 top = (bbmax - org) * idir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    range.x = max(max(max(tmin.x, tmin.y), tmin.z), range.x);
    range.y = min(min(min(tmax.x, tmax.y), tmax.z), range.y);

    return range.x <= range.y;
}
