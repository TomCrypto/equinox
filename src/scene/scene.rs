use crate::{
    Aperture, ApertureShape, Camera, Dirty, Display, Environment, Geometry, Instance, Integrator,
    Material, MaterialParameter, Metadata, Raster,
};
use js_sys::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! validate {
    ($cond: expr) => {
        if ($cond) == false {
            return Err(Error::new(&format!(
                "validation error: {}",
                stringify!($cond)
            )));
        }
    };

    ($prefix: expr, $cond: expr) => {
        if ($cond) == false {
            return Err(Error::new(&format!(
                "validation error: {}.{}",
                $prefix,
                stringify!($cond)
            )));
        }
    };
}

macro_rules! validate_contains {
    ($list: expr, $key: expr) => {
        if !$list.contains_key((&$key as &dyn AsRef<str>).as_ref()) {
            return Err(Error::new(&format!(
                "validation error: {} (`{}') not in {}",
                stringify!($key),
                $key,
                stringify!($list)
            )));
        }
    };

    ($list: expr, $prefix: expr, $key: expr) => {
        if !$list.contains_key((&$key as &dyn AsRef<str>).as_ref()) {
            return Err(Error::new(&format!(
                "validation error: {}.{} (`{}') not in {}",
                $prefix,
                stringify!($key),
                $key,
                stringify!($list)
            )));
        }
    };
}

/// # Dirty Flags
///
/// For pragmatic reasons, the scene structure maintains dirty flags relative to
/// a particular device instance's internal state. As a consequence care must be
/// taken when using the same scene instance on multiple devices simultaneously.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Scene {
    pub metadata: Dirty<Metadata>,
    pub camera: Dirty<Camera>,
    pub raster: Dirty<Raster>,
    pub instance_list: Dirty<BTreeMap<String, Instance>>,
    pub geometry_list: Dirty<BTreeMap<String, Geometry>>,
    pub material_list: Dirty<BTreeMap<String, Material>>,
    pub environment_map: Dirty<Option<String>>,
    pub environment: Dirty<Environment>,
    pub display: Dirty<Display>,
    pub aperture: Dirty<Option<Aperture>>,
    pub integrator: Dirty<Integrator>,
}

impl Scene {
    /// Marks the entire contents of this scene as dirty.
    ///
    /// This method will force a complete device update the next time the
    /// device is updated using this scene, and should be used sparingly.
    pub fn dirty_all_fields(&mut self) {
        Dirty::dirty(&mut self.metadata);
        Dirty::dirty(&mut self.camera);
        Dirty::dirty(&mut self.raster);
        Dirty::dirty(&mut self.instance_list);
        Dirty::dirty(&mut self.geometry_list);
        Dirty::dirty(&mut self.material_list);
        Dirty::dirty(&mut self.environment);
        Dirty::dirty(&mut self.environment_map);
        Dirty::dirty(&mut self.display);
        Dirty::dirty(&mut self.aperture);
        Dirty::dirty(&mut self.integrator);
    }

    /// Returns all referenced assets.
    pub fn assets(&self) -> Vec<&str> {
        let mut assets = vec![];

        if let Some(asset) = self.environment_map.as_ref() {
            assets.push(asset.as_str());
        }

        if let Some(aperture) = self.aperture.as_ref() {
            assets.push(&aperture.filter);
        }

        for material in self.material_list.values() {
            for (_, parameter) in material.parameters() {
                if let MaterialParameter::Textured(info) = parameter {
                    assets.push(info.texture.horz_texture());
                    assets.push(info.texture.vert_texture());
                }
            }
        }

        assets.sort_unstable();
        assets.dedup();
        assets
    }

    /// Patches this scene to be equal to another scene.
    ///
    /// Scene contents which are identical between the two scenes will not be
    /// modified, so the method will avoid dirtying as many fields as it can.
    pub fn patch_from_other(&mut self, other: Self) {
        if self.metadata != other.metadata {
            self.metadata = other.metadata;
        }

        if self.camera != other.camera {
            self.camera = other.camera;
        }

        if self.display != other.display {
            self.display = other.display;
        }

        if self.environment_map != other.environment_map {
            self.environment_map = other.environment_map;
        }

        if self.environment != other.environment {
            self.environment = other.environment;
        }

        if self.geometry_list != other.geometry_list {
            self.geometry_list = other.geometry_list;
        }

        if self.material_list != other.material_list {
            self.material_list = other.material_list;
        }

        if self.raster != other.raster {
            self.raster = other.raster;
        }

        if self.instance_list != other.instance_list {
            self.instance_list = other.instance_list;
        }

        if self.aperture != other.aperture {
            self.aperture = other.aperture;
        }

        if self.integrator != other.integrator {
            self.integrator = other.integrator;
        }
    }

    /// Validates all dirty contents of this scene.
    ///
    /// If this method succeeds, then the scene should always be renderable
    /// without errors, excluding device limitations and/or missing assets.
    pub fn validate(&self) -> Result<(), Error> {
        if let Some(metadata) = Dirty::as_dirty(&self.metadata) {
            self.validate_metadata(metadata)?;
        }

        if let Some(camera) = Dirty::as_dirty(&self.camera) {
            self.validate_camera(camera)?;
        }

        if let Some(raster) = Dirty::as_dirty(&self.raster) {
            self.validate_raster(raster)?;
        }

        if let Some(environment) = Dirty::as_dirty(&self.environment) {
            self.validate_environment(environment)?;
        }

        if let Some(display) = Dirty::as_dirty(&self.display) {
            self.validate_display(display)?;
        }

        if let Some(integrator) = Dirty::as_dirty(&self.integrator) {
            self.validate_integrator(integrator)?;
        }

        if let Some(instance_list) = Dirty::as_dirty(&self.instance_list) {
            self.validate_instance_list(instance_list)?;
        }

        if let Some(geometry_list) = Dirty::as_dirty(&self.geometry_list) {
            self.validate_geometry_list(geometry_list)?;
        }

        if let Some(material_list) = Dirty::as_dirty(&self.material_list) {
            self.validate_material_list(material_list)?;
        }

        Ok(())
    }

    pub(crate) fn has_photon_receivers(&self) -> bool {
        self.instance_list
            .values()
            .filter(|instance| instance.visible)
            .any(|instance| {
                if let Some(material) = self.material_list.get(&instance.material) {
                    material.is_photon_receiver()
                } else {
                    false
                }
            })
    }

    fn validate_metadata(&self, metadata: &Metadata) -> Result<(), Error> {
        validate!(metadata.name != "");

        Ok(())
    }

    fn validate_camera(&self, camera: &Camera) -> Result<(), Error> {
        validate!(camera.focal_distance > 0.0);
        validate!(camera.field_of_view > 0.0);
        validate!(camera.field_of_view <= 1.0);
        validate!(camera.focal_curvature >= 0.0);
        validate!(camera.direction != [0.0, 0.0, 0.0]);
        validate!(camera.up_vector != [0.0, 0.0, 0.0]);

        match camera.aperture {
            ApertureShape::Point => {}
            ApertureShape::Circle { radius } => {
                validate!("camera.aperture", radius >= 0.0);
                validate!("camera.aperture", radius <= 100.0);
            }
            ApertureShape::Ngon { radius, sides, .. } => {
                validate!("camera.aperture", radius >= 0.0);
                validate!("camera.aperture", radius <= 100.0);
                validate!("camera.aperture", sides >= 3);
            }
        }

        Ok(())
    }

    fn validate_raster(&self, raster: &Raster) -> Result<(), Error> {
        validate!(raster.width >= 1);
        validate!(raster.height >= 1);
        validate!(raster.width <= 8192);
        validate!(raster.height <= 8192);

        Ok(())
    }

    fn validate_environment(&self, environment: &Environment) -> Result<(), Error> {
        match environment {
            Environment::Solid { tint } | Environment::Map { tint, .. } => {
                validate!("environment", tint[0] >= 0.0);
                validate!("environment", tint[1] >= 0.0);
                validate!("environment", tint[2] >= 0.0);
            }
        }

        if let Environment::Map { .. } = environment {
            if self.environment_map.is_none() {
                return Err(Error::new("validation error: environment_map != null"));
            }
        }

        Ok(())
    }

    fn validate_display(&self, display: &Display) -> Result<(), Error> {
        validate!(display.exposure >= -10.0);
        validate!(display.exposure <= 10.0);
        validate!(display.saturation >= 0.0);
        validate!(display.saturation <= 1.0);
        validate!(display.lens_flare_tiles_per_pass > 0);

        if display.lens_flare_enabled && self.aperture.is_none() {
            return Err(Error::new("lens flare enabled with no aperture"));
        }

        Ok(())
    }

    fn validate_integrator(&self, integrator: &Integrator) -> Result<(), Error> {
        validate!(integrator.hash_table_bits >= 18);
        validate!(integrator.hash_table_bits <= 24);
        validate!(integrator.photons_per_pass > 0);
        validate!(integrator.max_search_radius > 0.0);
        validate!(integrator.min_search_radius > 0.0);
        validate!(integrator.alpha >= 0.0);
        validate!(integrator.alpha <= 1.0);
        validate!(integrator.max_scatter_bounces > 0);
        validate!(integrator.max_gather_bounces > 0);
        validate!(integrator.geometry_precision >= 1e-5);
        validate!(integrator.geometry_precision <= 1e-2);
        validate!(integrator.geometry_pushback >= 2.0);

        Ok(())
    }

    fn validate_instance_list(
        &self,
        instance_list: &BTreeMap<String, Instance>,
    ) -> Result<(), Error> {
        let geometry_list = &self.geometry_list;
        let material_list = &self.material_list;

        for (
            name,
            Instance {
                geometry,
                material,
                parameters,
                parent,
                medium,
                ..
            },
        ) in instance_list.iter()
        {
            let prefix = format!("instance_list[\"{}\"]", name);

            validate_contains!(geometry_list, prefix, geometry);
            validate_contains!(material_list, prefix, material);

            if let Some(parent) = parent {
                validate_contains!(instance_list, prefix, parent);
            }

            validate!(prefix, medium.extinction[0] >= 0.0);
            validate!(prefix, medium.extinction[1] >= 0.0);
            validate!(prefix, medium.extinction[2] >= 0.0);
            validate!(prefix, medium.refractive_index >= 1.0);

            for parameter in geometry_list[geometry].symbolic_parameters() {
                if !parameters.contains_key(parameter) {
                    let geometry_prefix = format!("geometry_list[\"{}\"]", geometry);

                    return Err(Error::new(&format!(
                        "validation error: {} parameter `{}' missing in {}.parameters",
                        geometry_prefix, parameter, prefix
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_geometry_list(
        &self,
        geometry_list: &BTreeMap<String, Geometry>,
    ) -> Result<(), Error> {
        for (name, _geometry) in geometry_list.iter() {
            let _prefix = format!("geometry_list[\"{}\"]", name);

            // TODO: implement geometry validation
        }

        Ok(())
    }

    fn validate_material_list(
        &self,
        material_list: &BTreeMap<String, Material>,
    ) -> Result<(), Error> {
        for (name, material) in material_list.iter() {
            for (parameter_name, parameter) in material.parameters() {
                if let MaterialParameter::Textured(info) = parameter {
                    let prefix = format!("material_list[\"{}\"].{}", name, parameter_name);

                    let contrast = info.contrast;

                    validate!(prefix, contrast >= 0.0);
                    validate!(prefix, contrast <= 1.0);
                }
            }
        }

        Ok(())
    }
}
