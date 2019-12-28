#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{Device, Material, MaterialParameter};
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
        Material::Lambertian { .. } => 1,
        Material::IdealReflection { .. } => 1,
        Material::IdealRefraction { .. } => 1,
        Material::Phong { .. } => 2,
        Material::Dielectric { .. } => 1,
        Material::OrenNayar { .. } => 2,
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
            let horz = texture_layers[info.texture.horz_texture()] as u32;
            let vert = texture_layers[info.texture.vert_texture()] as u32;

            out.layer = vert + (horz << 16);
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

impl Device {
    const MATERIAL_TEXTURE_COLS: usize = 2048;
    const MATERIAL_TEXTURE_ROWS: usize = 2048;

    fn textures_out_of_date(&self, textures: &[&str]) -> bool {
        if self.loaded_textures.len() != textures.len() {
            return true;
        }

        !textures.iter().eq(self.loaded_textures.iter())
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

        if self.material_textures.is_invalid() || self.textures_out_of_date(textures) {
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

            self.loaded_textures = textures.iter().map(|&texture| texture.to_owned()).collect();
        }

        Ok(())
    }

    pub(crate) fn update_materials(
        &mut self,
        materials: &BTreeMap<String, Material>,
        assets: &dyn Fn(&str) -> Result<Vec<u8>, Error>,
    ) -> Result<(), Error> {
        let mut parameter_count = 0;
        let mut textures = vec![];

        for material in materials.values() {
            parameter_count += material_parameter_count(material);

            for (_, parameter) in material.parameters() {
                if let MaterialParameter::Textured(info) = parameter {
                    textures.push(info.texture.horz_texture());
                    textures.push(info.texture.vert_texture());
                }
            }
        }

        textures.sort_unstable();
        textures.dedup();

        let mut texture_layers = BTreeMap::new();

        for (index, &texture) in textures.iter().enumerate() {
            texture_layers.insert(texture, index);
        }

        self.upload_material_textures(assets, &textures)?;

        let mut parameters = vec![MaterialParamData::default(); parameter_count];
        let mut start = 0;

        for material in materials.values() {
            let count = material_parameter_count(material);

            for (index, (_, parameter)) in material.parameters().into_iter().enumerate() {
                write_material_parameter(
                    parameter,
                    &mut parameters[start + index],
                    &texture_layers,
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
