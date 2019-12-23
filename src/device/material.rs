#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{Device, Material, MaterialParameter, TextureMapping};
use js_sys::Error;
use std::collections::BTreeMap;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Clone, Copy, Debug, Default)]
pub struct MaterialParameterData {
    base: [f32; 4],
    scale: [f32; 4],
    texture: u32,
    mapping_scale: f32,
    mapping_offset: [f32; 2],
}

pub(crate) fn material_index(material: &Material) -> u16 {
    match material {
        Material::Lambertian { .. } => 0,
        Material::IdealReflection { .. } => 1,
        Material::Phong { .. } => 2,
        Material::IdealRefraction { .. } => 3,
        Material::Dielectric { .. } => 4,
        Material::OrenNayar { .. } => 5,
    }
}

/// Returns the number of parameters used by a material.
pub(crate) fn material_parameter_count(material: &Material) -> usize {
    match material {
        Material::Lambertian { .. } => 1,
        Material::IdealReflection { .. } => 1,
        Material::IdealRefraction { .. } => 1,
        Material::Phong { .. } => 2,
        Material::Dielectric { .. } => 1,
        Material::OrenNayar { .. } => 2,
    }
}

// TODO: refactor this to remove duplication

// TODO: pass texture layer mapping here
fn write_material_parameter_float(param: &MaterialParameter<f32>, out: &mut MaterialParameterData) {
    out.base[0] = param.base();
    out.scale[0] = param.scale();

    if let Some((texture, mapping)) = param.texture() {
        // TODO: look up texture layer from mapping
        // and set the scale/offset appropriately

        out.texture = 0;

        match mapping {
            TextureMapping::Triplanar { scale, offset } => {
                out.mapping_scale = *scale;
                out.mapping_offset = *offset;
            }
            TextureMapping::TriplanarStochastic { scale, offset } => {
                out.mapping_scale = *scale;
                out.mapping_offset = *offset;
            }
        }
    } else {
        out.texture = 0;
    }
}

fn write_material_parameter_vec3(
    param: &MaterialParameter<[f32; 3]>,
    out: &mut MaterialParameterData,
) {
    let base = param.base();
    let scale = param.scale();

    out.base[0] = base[0];
    out.base[1] = base[1];
    out.base[2] = base[2];
    out.scale[0] = scale[0];
    out.scale[1] = scale[1];
    out.scale[2] = scale[2];

    if let Some((texture, mapping)) = param.texture() {
        // TODO: look up texture layer from mapping
        // and set the scale/offset appropriately

        out.texture = 0;

        match mapping {
            TextureMapping::Triplanar { scale, offset } => {
                out.mapping_scale = *scale;
                out.mapping_offset = *offset;
            }
            TextureMapping::TriplanarStochastic { scale, offset } => {
                out.mapping_scale = *scale;
                out.mapping_offset = *offset;
            }
        }
    } else {
        out.texture = 0;
    }
}

fn write_material_parameters(material: &Material, parameters: &mut [MaterialParameterData]) {
    match material {
        Material::Lambertian { albedo } => {
            write_material_parameter_vec3(albedo, &mut parameters[0]);
        }
        Material::IdealReflection { reflectance } => {
            write_material_parameter_vec3(reflectance, &mut parameters[0]);
        }
        Material::IdealRefraction { transmittance } => {
            write_material_parameter_vec3(transmittance, &mut parameters[0]);
        }
        Material::Phong { albedo, shininess } => {
            write_material_parameter_vec3(albedo, &mut parameters[0]);
            write_material_parameter_float(shininess, &mut parameters[1]);
        }
        Material::Dielectric { base_color } => {
            write_material_parameter_vec3(base_color, &mut parameters[0]);
        }
        Material::OrenNayar { albedo, roughness } => {
            write_material_parameter_vec3(albedo, &mut parameters[0]);
            write_material_parameter_float(roughness, &mut parameters[1]);
        }
    }
}

impl Device {
    pub(crate) fn update_materials(
        &mut self,
        materials: &BTreeMap<String, Material>,
    ) -> Result<(), Error> {
        // TODO: here, gather all of the textures and upload them to the appropriate
        // texture layer
        // how do we avoid constantly rebuilding this?

        let mut parameter_count = 0;

        for material in materials.values() {
            parameter_count += material_parameter_count(material);
        }

        let mut parameters = vec![MaterialParameterData::default(); parameter_count];
        let mut start = 0;

        for material in materials.values() {
            let count = material_parameter_count(material);

            // TODO: pass in the texture string -> layer mapping to this function...

            write_material_parameters(material, &mut parameters[start..start + count]);

            start += count;
        }

        self.material_buffer
            .write_array(self.material_buffer.max_len(), &parameters)?;
        self.integrator_gather_photons_shader
            .set_define("MATERIAL_DATA_LEN", self.material_buffer.len());
        self.integrator_scatter_photons_shader
            .set_define("MATERIAL_DATA_LEN", self.material_buffer.len());

        Ok(())
    }
}
