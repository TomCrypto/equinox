use crate::{Camera, Dirty, Display, Environment, Geometries, Instances, Materials, Raster};

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
    pub instances: Dirty<Instances>,
    pub geometries: Dirty<Geometries>,
    pub materials: Dirty<Materials>,
    pub environment: Dirty<Environment>,
    pub display: Dirty<Display>,
}

impl Scene {
    /// Marks the entire contents of this scene as dirty.
    ///
    /// This method will force a complete device update the next time that a
    /// device is updated using this scene, and so should be used sparingly.
    pub fn dirty_all_fields(&mut self) {
        Dirty::dirty(&mut self.camera);
        Dirty::dirty(&mut self.raster);
        Dirty::dirty(&mut self.instances);
        Dirty::dirty(&mut self.geometries);
        Dirty::dirty(&mut self.materials);
        Dirty::dirty(&mut self.environment);
        Dirty::dirty(&mut self.display);
    }
}
