#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::Material;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct MaterialParameter([f32; 4]);

pub(crate) fn material_index(material: &Material) -> u16 {
    match material {
        Material::Lambertian { .. } => 0,
        Material::IdealReflection { .. } => 1,
        Material::Phong { .. } => 2,
        Material::IdealRefraction { .. } => 3,
        Material::Dielectric { .. } => 4,
    }
}

/// Returns the number of 4-float parameter blocks used by a material.
pub(crate) fn material_parameter_block_count(material: &Material) -> usize {
    match material {
        Material::Lambertian { .. } => 1,
        Material::IdealReflection { .. } => 1,
        Material::IdealRefraction { .. } => 1,
        Material::Phong { .. } => 1,
        Material::Dielectric { .. } => 2,
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
        Material::Phong { albedo, shininess } => {
            parameters[0].0[0] = albedo[0];
            parameters[0].0[1] = albedo[1];
            parameters[0].0[2] = albedo[2];
            parameters[0].0[3] = *shininess;
        }
        Material::Dielectric {
            internal_refractive_index,
            external_refractive_index,
            internal_extinction_coefficient,
            external_extinction_coefficient,
        } => {
            parameters[0].0[0] = internal_extinction_coefficient[0];
            parameters[0].0[1] = internal_extinction_coefficient[1];
            parameters[0].0[2] = internal_extinction_coefficient[2];
            parameters[0].0[3] = *internal_refractive_index;
            parameters[1].0[0] = external_extinction_coefficient[0];
            parameters[1].0[1] = external_extinction_coefficient[1];
            parameters[1].0[2] = external_extinction_coefficient[2];
            parameters[1].0[3] = *external_refractive_index;
        }
    }
}

impl Device {
    pub(crate) fn update_materials(&mut self, materials: &[Material]) {
        let mut block_count = 0;

        for material in materials {
            block_count += material_parameter_block_count(material);
        }

        let parameters: &mut [MaterialParameter] = self.allocator.allocate(block_count);
        let mut start = 0;

        for material in materials {
            let count = material_parameter_block_count(material);

            write_material_parameters(material, &mut parameters[start..start + count]);

            start += count;
        }

        assert!(parameters.len() <= self.material_buffer.max_len());
        self.material_buffer.write_array(&parameters);
        self.program
            .set_define("MATERIAL_DATA_COUNT", self.material_buffer.element_count());
    }
}
