use crate::{Aperture, Camera, Dirty, Display, Environment, Geometry, Instance, Material, Raster};

use serde::{Deserialize, Serialize};

/// # Dirty Flags
///
/// For pragmatic reasons, the scene structure maintains dirty flags relative to
/// a particular device instance's internal state. As a consequence care must be
/// taken when using the same scene instance on multiple devices simultaneously.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Scene {
    pub camera: Dirty<Camera>,
    pub raster: Dirty<Raster>,
    pub instances: Dirty<Vec<Instance>>,
    pub geometries: Dirty<Vec<Geometry>>,
    pub materials: Dirty<Vec<Material>>,
    pub environment: Dirty<Environment>,
    pub display: Dirty<Display>,
    pub aperture: Dirty<Aperture>,
}

impl Scene {
    /// Marks the entire contents of this scene as dirty.
    ///
    /// This method will force a complete device update the next time the
    /// device is updated using this scene, and should be used sparingly.
    pub fn dirty_all_fields(&mut self) {
        Dirty::dirty(&mut self.camera);
        Dirty::dirty(&mut self.raster);
        Dirty::dirty(&mut self.instances);
        Dirty::dirty(&mut self.geometries);
        Dirty::dirty(&mut self.materials);
        Dirty::dirty(&mut self.environment);
        Dirty::dirty(&mut self.display);
        Dirty::dirty(&mut self.aperture);
    }
}