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
    vec4 hit;       // (minimum ray distance, maximum ray distance, barycentric u, barycentric v)
    uvec4 triangle; // (absolute vertex offsets, absolute material offset) for closest triangle
};

bool ray_bbox(vec3 org, vec3 idir, vec3 bmin, vec3 bmax, in traversal_t traversal) {
    vec3 bot = (bmin - org) * idir;
    vec3 top = (bmax - org) * idir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    float near = max(max(tmin.x, tmin.y), tmin.z);
    float far = min(min(tmax.x, tmax.y), tmax.z);

    return (near <= far) && (far > traversal.hit.x) && (near < traversal.hit.y);
}
