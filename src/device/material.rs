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
fn write_material_parameter_float(
    param: &MaterialParameter<f32>,
    out: &mut MaterialParameterData,
    texture_layers: &BTreeMap<&str, usize>,
) {
    out.base[0] = param.base();
    out.scale[0] = param.scale();

    if let Some((texture, mapping)) = param.texture() {
        // TODO: add texture mapping mode (stochastic or not) in the texture index
        // somehow

        out.texture = texture_layers[texture] as u32;

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
    texture_layers: &BTreeMap<&str, usize>,
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
        // TODO: add texture mapping mode (stochastic or not) in the texture index
        // somehow

        out.texture = texture_layers[texture] as u32;

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

fn write_material_parameters(
    material: &Material,
    parameters: &mut [MaterialParameterData],
    texture_layers: &BTreeMap<&str, usize>,
) {
    match material {
        Material::Lambertian { albedo } => {
            write_material_parameter_vec3(albedo, &mut parameters[0], texture_layers);
        }
        Material::IdealReflection { reflectance } => {
            write_material_parameter_vec3(reflectance, &mut parameters[0], texture_layers);
        }
        Material::IdealRefraction { transmittance } => {
            write_material_parameter_vec3(transmittance, &mut parameters[0], texture_layers);
        }
        Material::Phong { albedo, shininess } => {
            write_material_parameter_vec3(albedo, &mut parameters[0], texture_layers);
            write_material_parameter_float(shininess, &mut parameters[1], texture_layers);
        }
        Material::Dielectric { base_color } => {
            write_material_parameter_vec3(base_color, &mut parameters[0], texture_layers);
        }
        Material::OrenNayar { albedo, roughness } => {
            write_material_parameter_vec3(albedo, &mut parameters[0], texture_layers);
            write_material_parameter_float(roughness, &mut parameters[1], texture_layers);
        }
    }
}

impl Device {
    fn textures_out_of_date(&self, textures: &[&str]) -> bool {
        if self.loaded_textures.len() != textures.len() {
            return true;
        }

        textures.iter().eq(self.loaded_textures.iter())
    }

    fn upload_material_textures(&mut self, textures: &[&str]) {
        if
        /* texture array is invalidated || */
        self.textures_out_of_date(textures) {
            // recreate entire texture array!

            self.loaded_textures = textures.iter().map(|&texture| texture.to_owned()).collect();
        }
    }

    fn add_texture<'a>(texture: Option<&'a str>, textures: &mut Vec<&'a str>) {
        if let Some(texture) = texture {
            textures.push(texture);
        }
    }

    pub(crate) fn update_materials(
        &mut self,
        materials: &BTreeMap<String, Material>,
    ) -> Result<(), Error> {
        let mut parameter_count = 0;
        let mut textures = vec![];

        for material in materials.values() {
            parameter_count += material_parameter_count(material);

            match material {
                Material::Lambertian { albedo } => {
                    Self::add_texture(albedo.texture2(), &mut textures);
                }
                Material::IdealReflection { reflectance } => {
                    Self::add_texture(reflectance.texture2(), &mut textures);
                }
                Material::IdealRefraction { transmittance } => {
                    Self::add_texture(transmittance.texture2(), &mut textures);
                }
                Material::Phong { albedo, shininess } => {
                    Self::add_texture(albedo.texture2(), &mut textures);
                    Self::add_texture(shininess.texture2(), &mut textures);
                }
                Material::Dielectric { base_color } => {
                    Self::add_texture(base_color.texture2(), &mut textures);
                }
                Material::OrenNayar { albedo, roughness } => {
                    Self::add_texture(albedo.texture2(), &mut textures);
                    Self::add_texture(roughness.texture2(), &mut textures);
                }
            }
        }

        textures.sort_unstable();
        textures.dedup();

        let mut texture_layers = BTreeMap::new();

        for (index, &texture) in textures.iter().enumerate() {
            texture_layers.insert(texture, index);
        }

        self.upload_material_textures(&textures);

        let mut parameters = vec![MaterialParameterData::default(); parameter_count];
        let mut start = 0;

        for material in materials.values() {
            let count = material_parameter_count(material);

            write_material_parameters(
                material,
                &mut parameters[start..start + count],
                &texture_layers,
            );

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
