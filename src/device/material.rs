#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{Device, Material, MaterialParameter, NormalMapParameter};
use img2raw::{ColorSpace, DataFormat, Header};
use js_sys::Error;
use std::collections::BTreeMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Clone, Copy, Debug, Default)]
pub struct MaterialParamData {
    base: [f32; 3],
    layer: u32,
    factor: [f32; 3],
    contrast: f32,
    uv_rotation: f32,
    uv_scale: f32,
    uv_offset: [f32; 2],
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
        Material::Lambertian { .. } => 2,
        Material::IdealReflection { .. } => 2,
        Material::IdealRefraction { .. } => 2,
        Material::Phong { .. } => 3,
        Material::Dielectric { .. } => 2,
        Material::OrenNayar { .. } => 3,
    }
}

fn write_material_parameter(
    parameter: &MaterialParameter,
    out: &mut MaterialParamData,
    texture_layers: &BTreeMap<&str, usize>,
) {
    match parameter {
        MaterialParameter::Constant(base) => {
            out.layer = 0xffff_ffff;
            out.base = base.as_vec3();
            out.factor = [0.0; 3];
        }
        MaterialParameter::Textured(info) => {
            out.layer = texture_layers[info.texture.as_str()] as u32;
            out.base = info.base.as_vec3();
            out.factor = info.factor.as_vec3();

            out.uv_scale = info.uv_scale;
            out.uv_offset = info.uv_offset;
            out.uv_rotation = info.uv_rotation.rem_euclid(2.0 * std::f32::consts::PI);
            out.contrast = info.contrast * 2.0;

            if !info.stochastic {
                out.contrast *= -1.0;
            }
        }
    }
}

fn write_normal_map_parameter(
    parameter: Option<&NormalMapParameter>,
    out: &mut MaterialParamData,
    texture_layers: &BTreeMap<&str, usize>,
) {
    if let Some(parameter) = parameter {
        out.layer = texture_layers[parameter.texture.as_str()] as u32;

        out.uv_scale = parameter.uv_scale;
        out.uv_offset = parameter.uv_offset;
        out.uv_rotation = parameter.uv_rotation.rem_euclid(2.0 * std::f32::consts::PI);

        out.base[0] = parameter.strength;

        // TODO: stochastic/constrast?
    } else {
        out.layer = 0xffff_ffff;
    }
}

impl Device {
    const MATERIAL_TEXTURE_COLS: usize = 2048;
    const MATERIAL_TEXTURE_ROWS: usize = 2048;

    fn material_textures_out_of_date(&self, textures: &[&str]) -> bool {
        if self.loaded_material_textures.len() != textures.len() {
            return true;
        }

        !textures.iter().eq(self.loaded_material_textures.iter())
    }

    fn normal_textures_out_of_date(&self, textures: &[&str]) -> bool {
        if self.loaded_normal_textures.len() != textures.len() {
            return true;
        }

        !textures.iter().eq(self.loaded_normal_textures.iter())
    }

    fn upload_material_textures(
        &mut self,
        assets: &dyn Fn(&str) -> Result<Vec<u8>, Error>,
        textures: &[&str],
    ) -> Result<(), Error> {
        // TODO: in the future, add support for ASTC compression for mobile. This
        // involves having a separate texture array for that compression format,
        // validating the asset format below, and hooking everything together.
        // The front-end will have to be responsible for providing the assets in the
        // right format, the details of which this renderer is not concerned with.

        if self.material_textures.is_invalid() || self.material_textures_out_of_date(textures) {
            if textures.is_empty() {
                self.material_textures.reset();
            } else {
                self.material_textures.create_array_compressed(
                    Self::MATERIAL_TEXTURE_COLS,
                    Self::MATERIAL_TEXTURE_ROWS,
                    textures.len(),
                )?;

                for (layer, texture) in textures.iter().enumerate() {
                    let asset_data = assets(texture)?;

                    let (header, data) =
                        LayoutVerified::<_, Header>::new_from_prefix(asset_data.as_slice())
                            .unwrap();

                    if header.data_format.try_parse() != Some(DataFormat::BC1) {
                        return Err(Error::new("expected BC1 material texture"));
                    }

                    if header.color_space.try_parse() != Some(ColorSpace::SRGB) {
                        return Err(Error::new("expected sRGB material texture"));
                    }

                    if header.dimensions[0] as usize != Self::MATERIAL_TEXTURE_COLS {
                        return Err(Error::new("invalid material texture dimensions"));
                    }

                    if header.dimensions[1] as usize != Self::MATERIAL_TEXTURE_ROWS {
                        return Err(Error::new("invalid material texture dimensions"));
                    }

                    self.material_textures.upload_layer_compressed(
                        Self::MATERIAL_TEXTURE_COLS,
                        Self::MATERIAL_TEXTURE_ROWS,
                        layer,
                        &data,
                    );
                }
            }

            self.loaded_material_textures = textures.iter().map(|&texture| texture.to_owned()).collect();
        }

        Ok(())
    }

    fn upload_normal_textures(
        &mut self,
        assets: &dyn Fn(&str) -> Result<Vec<u8>, Error>,
        textures: &[&str],
    ) -> Result<(), Error> {
        if self.normal_textures.is_invalid() || self.normal_textures_out_of_date(textures) {
            if textures.is_empty() {
                self.normal_textures.reset();
            } else {
                self.normal_textures.create_array(
                    Self::MATERIAL_TEXTURE_COLS,
                    Self::MATERIAL_TEXTURE_ROWS,
                    textures.len(),
                );

                for (layer, texture) in textures.iter().enumerate() {
                    let asset_data = assets(texture)?;

                    let (header, data) =
                        LayoutVerified::<_, Header>::new_from_prefix(asset_data.as_slice())
                            .unwrap();

                    if header.data_format.try_parse() != Some(DataFormat::RG8) {
                        return Err(Error::new("expected RG8 normal texture"));
                    }

                    if header.color_space.try_parse() != Some(ColorSpace::NonColor) {
                        return Err(Error::new("expected non-color normal texture"));
                    }

                    if header.dimensions[0] as usize != Self::MATERIAL_TEXTURE_COLS {
                        return Err(Error::new("invalid normal texture dimensions"));
                    }

                    if header.dimensions[1] as usize != Self::MATERIAL_TEXTURE_ROWS {
                        return Err(Error::new("invalid normal texture dimensions"));
                    }

                    self.normal_textures.upload_layer(
                        Self::MATERIAL_TEXTURE_COLS,
                        Self::MATERIAL_TEXTURE_ROWS,
                        layer,
                        &data,
                    );
                }
            }

            self.loaded_normal_textures = textures.iter().map(|&texture| texture.to_owned()).collect();
        }

        Ok(())
    }

    pub(crate) fn update_materials(
        &mut self,
        materials: &BTreeMap<String, Material>,
        assets: &dyn Fn(&str) -> Result<Vec<u8>, Error>,
    ) -> Result<(), Error> {
        let mut parameter_count = 0;
        let mut material_textures: Vec<&str> = vec![];
        let mut normal_textures: Vec<&str> = vec![];

        for material in materials.values() {
            parameter_count += material_parameter_count(material);

            if let Some(parameter) = material.normal_map() {
                normal_textures.push(&parameter.texture);
            }

            for (_, parameter) in material.parameters() {
                if let MaterialParameter::Textured(info) = parameter {
                    material_textures.push(&info.texture);
                }
            }
        }

        material_textures.sort_unstable();
        material_textures.dedup();
        normal_textures.sort_unstable();
        normal_textures.dedup();

        let mut material_texture_layers = BTreeMap::new();
        let mut normal_texture_layers = BTreeMap::new();

        for (index, &texture) in material_textures.iter().enumerate() {
            material_texture_layers.insert(texture, index);
        }

        for (index, &texture) in normal_textures.iter().enumerate() {
            normal_texture_layers.insert(texture, index);
        }

        self.upload_material_textures(assets, &material_textures)?;
        self.upload_normal_textures(assets, &normal_textures)?;

        let mut parameters = vec![MaterialParamData::default(); parameter_count];
        let mut start = 0;

        for material in materials.values() {
            let count = material_parameter_count(material);

            write_normal_map_parameter(material.normal_map(), &mut parameters[start], &normal_texture_layers);
            
            for (index, (_, parameter)) in material.parameters().into_iter().enumerate() {
                write_material_parameter(
                    parameter,
                    &mut parameters[start + index + 1],
                    &material_texture_layers,
                );
            }

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
