in vec3 photon_pos_data;
in vec3 photon_dir_data;
in vec3 photon_sum_data;

layout (location = 0) out vec4 photon_pos;
layout (location = 1) out vec4 photon_dir;
layout (location = 2) out vec4 photon_sum;

void main() {
    photon_pos = vec4(photon_pos_data, 1.0);
    photon_dir = vec4(photon_dir_data, 1.0);
    photon_sum = vec4(photon_sum_data, 0.0);
}
