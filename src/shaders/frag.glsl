#include <common.glsl>
#include <random.glsl>

#include <material.glsl>

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
    float pass_count;
    float photons_for_pass;
    float total_photons;
    float grid_cell_size;
    uint hash_cell_cols;
    uint hash_cell_rows;
    uint hash_cell_col_bits;
    float alpha;
} globals;

uniform sampler2D visible_point_path_buf1;
uniform sampler2D visible_point_path_buf2;
uniform sampler2D visible_point_path_buf3;

// direct lighting consists of:
//  - any path consisting of only non-receiving surfaces (no point to gather photons from)
//  - any path of the form camera - receiving surface - light ("explicit light sampling")

out vec4 result;

uniform sampler2D photon_table_major;
uniform sampler2D photon_table_minor;
uniform sampler2D photon_radius_tex;

#define FRAME_RANDOM (globals.frame_state.xy)

void main() {
    
}
