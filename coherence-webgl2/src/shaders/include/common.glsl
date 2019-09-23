struct ray_t {
    vec3 org;
    vec3 dir;
};

// Maintains closest-hit information during a traversal.
struct traversal_t {
    vec4 hit;       // (minimum ray distance, maximum ray distance, barycentric u, barycentric v)
    // uvec4 triangle; // (absolute vertex offsets, absolute material offset) for closest triangle
    uint triangle;
};

bool ray_bbox(vec3 org, vec3 idir, vec3 bmin, vec3 bmax, in traversal_t traversal) {
    vec3 bot = (bmin - org) * idir;
    vec3 top = (bmax - org) * idir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    float near = max(max(tmin.x, tmin.y), tmin.z);
    float far = min(min(tmax.x, tmax.y), tmax.z);

    if (near > far) {
        return false;
    }

    // if tmin < 0 => we're either inside the box or the box is behind us
    
    // if near < 0 AND far < 0, FALSE (AABB is behind us)
    // if near < 0 AND far > hit.x, TRUE
    // if near > hit.x AND near < hit.y, TRUE
    // else, FALSE

    if (max(near, traversal.hit.x) <= min(far, traversal.hit.y)) {
        return true;
    } else {
        return false;
    }

    // we have an intersection in two cases:
    // tmin < 0 (we're inside the box)
    // 

    // now the closest is tnear, unless it's negative then we're inside the box (so we're OK)
    // so the actual check should be:
    // tmin < 0 OR tmin < hit.y
    // i.e. tmin < hit.y

    // return (near < traversal.hit.y);

    // return (near <= far) && (far > traversal.hit.x) && (near < traversal.hit.y);
}

// this will return the "near" distance from the ray to the AABB, or zero if the AABB is behind
// the ray or the ray is contained within the AABB (rare case). this can be used to sort boxes
// for traversal
bool ray_bbox_with_distance(vec3 org, vec3 idir, vec3 bmin, vec3 bmax, in traversal_t traversal, out float distance) {
    vec3 bot = (bmin - org) * idir;
    vec3 top = (bmax - org) * idir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    float near = max(max(tmin.x, tmin.y), tmin.z);
    float far = min(min(tmax.x, tmax.y), tmax.z);

    if (near > far) {
        return false;
    }

    if (max(near, traversal.hit.x) <= min(far, traversal.hit.y)) {
        distance = near;
        return true;
    } else {
        return false;
    }
}
