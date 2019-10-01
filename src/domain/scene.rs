use crate::{Camera, Dirty, Display, Environment, Geometries, Instances, Materials, Raster};

use serde::{Deserialize, Serialize};

/// # Dirty Flags
///
/// For pragmatic reasons, the scene structure maintains dirty flags relative to
/// a particular device instance's internal state. As a consequence care must be
/// taken when rendering a scene on multiple devices simultaneously.
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
    /// Creates a new empty scene with a default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks all of this scene as dirty, forcing a complete device update.
    ///
    /// This is normally only used internally by devices to respond to events
    /// such as device loss. However because the dirty flags stored by scenes
    /// are associated with a device's current state, you should call this if
    /// a scene is "moved" from one device to another (not recommended).
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
