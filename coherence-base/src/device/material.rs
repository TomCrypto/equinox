use crate::device::ToDevice;
use crate::model::{Material, Materials};
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(AsBytes, FromBytes, Copy, Clone)]
pub struct MaterialData {
    kind: f32,
    color: [f32; 3],
}

impl ToDevice<[MaterialData]> for Materials {
    fn to_device(&self, slice: &mut [MaterialData]) {
        for (index, material) in self.list.iter().enumerate() {
            match material {
                Material::Diffuse { color } => {
                    slice[index].kind = 0.0;
                    slice[index].color = [color.x, color.y, color.z];
                }
                Material::Specular => {
                    slice[index].kind = 1.0;
                }
                Material::Emissive { strength } => {
                    slice[index].kind = 2.0;
                    slice[index].color = [*strength, *strength, *strength];
                }
            }
        }
    }

    fn requested_count(&self) -> usize {
        self.list.len()
    }
}
