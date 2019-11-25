use crate::{
    Aperture, Camera, Dirty, Display, Environment, Geometry, Instance, Integrator, Material, Raster,
};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub type Asset = String;

/// # Dirty Flags
///
/// For pragmatic reasons, the scene structure maintains dirty flags relative to
/// a particular device instance's internal state. As a consequence care must be
/// taken when using the same scene instance on multiple devices simultaneously.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Scene {
    pub camera: Dirty<Camera>,
    pub raster: Dirty<Raster>,
    pub instance_list: Dirty<BTreeMap<String, Instance>>,
    pub geometry_list: Dirty<BTreeMap<String, Geometry>>,
    pub material_list: Dirty<BTreeMap<String, Material>>,
    pub environment_map: Dirty<Option<Asset>>,
    pub environment: Dirty<Environment>,
    pub display: Dirty<Display>,
    pub aperture: Dirty<Option<Aperture>>,
    pub integrator: Dirty<Integrator>,

    #[serde(skip)]
    pub assets: HashMap<Asset, Vec<u8>>,
}

impl Scene {
    /// Marks the entire contents of this scene as dirty.
    ///
    /// This method will force a complete device update the next time the
    /// device is updated using this scene, and should be used sparingly.
    pub fn dirty_all_fields(&mut self) {
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

    /// Patches this scene to be equal to another scene.
    ///
    /// Scene contents which are identical between the two scenes will not be
    /// modified, so the method will avoid dirtying as many fields as it can.
    pub fn patch_from_other(&mut self, other: Self) {
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

    /// Returns a list of all assets actually used in the scene.
    ///
    /// It is possible for assets to be preloaded for the scene without being
    /// referenced anywhere; this method will detect which assets are in use.
    pub fn used_assets(&self) -> Vec<Asset> {
        self.assets
            .keys()
            .filter(|&asset| {
                if self.environment_map.as_ref() == Some(asset) {
                    return true;
                }

                if let Some(aperture) = &*self.aperture {
                    if &aperture.aperture_texels == asset {
                        return true;
                    }
                }

                false
            })
            .cloned()
            .collect()
    }

    pub fn has_photon_receivers(&self) -> bool {
        self.instance_list
            .values()
            .filter(|instance| instance.visible && instance.photon_receiver)
            .any(|instance| {
                if let Some(material) = self.material_list.get(&instance.material) {
                    !material.has_delta_bsdf()
                } else {
                    false
                }
            })
    }
}
