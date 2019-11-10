in vec4 photon_major_data;
in vec4 photon_minor_data;

layout (location = 0) out vec4 photon_major;
layout (location = 1) out vec4 photon_minor;

void main() {
    photon_major = photon_major_data;
    photon_minor = photon_minor_data;
}
