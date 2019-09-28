#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use coherence_base::{Material, Materials};
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct MaterialParameter([f32; 4]);

pub fn material_index(material: &Material) -> u16 {
    match material {
        Material::Lambertian { .. } => 0,
        Material::IdealReflection { .. } => 1,
        Material::Phong { .. } => 2,
        Material::IdealRefraction { .. } => 3,
    }
}

/// Returns the number of 4-float parameter blocks used by a material.
pub fn material_parameter_block_count(material: &Material) -> usize {
    match material {
        Material::Lambertian { .. } => 1,
        Material::IdealReflection { .. } => 1,
        Material::IdealRefraction { .. } => 1,
        Material::Phong { .. } => 2,
    }
}

fn write_material_parameters(material: &Material, parameters: &mut [MaterialParameter]) {
    match material {
        Material::Lambertian { albedo } => {
            parameters[0].0[0] = albedo[0];
            parameters[0].0[1] = albedo[1];
            parameters[0].0[2] = albedo[2];
        }
        Material::IdealReflection { reflectance } => {
            parameters[0].0[0] = reflectance[0];
            parameters[0].0[1] = reflectance[1];
            parameters[0].0[2] = reflectance[2];
        }
        Material::IdealRefraction {
            transmittance,
            refractive_index,
        } => {
            parameters[0].0[0] = transmittance[0];
            parameters[0].0[1] = transmittance[1];
            parameters[0].0[2] = transmittance[2];
            parameters[0].0[3] = *refractive_index;
        }
        Material::Phong {
            albedo,
            shininess,
            kd,
        } => {
            parameters[0].0[0] = albedo[0];
            parameters[0].0[1] = albedo[1];
            parameters[0].0[2] = albedo[2];
            parameters[0].0[3] = *kd;
            parameters[1].0[0] = *shininess;
        }
    }
}

impl Device {
    pub(crate) fn update_materials(&mut self, materials: &Materials) {
        let mut block_count = 0;

        for material in &materials.list {
            block_count += material_parameter_block_count(material);
        }

        let parameters: &mut [MaterialParameter] = self.scratch.allocate(block_count);
        let mut start = 0;

        for material in &materials.list {
            let count = material_parameter_block_count(material);

            write_material_parameters(material, &mut parameters[start..start + count]);

            start += count;
        }

        info!("MATERIAL PARAMS = {:?}", parameters);

        self.material_buffer.write_array(&parameters);
    }
}
