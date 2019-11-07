in vec4 table_major;
in vec4 table_minor;

layout (location = 0) out vec4 photon_major;
layout (location = 1) out vec4 photon_minor;

void main() {
    photon_major = table_major;
    photon_minor = table_minor;
}
