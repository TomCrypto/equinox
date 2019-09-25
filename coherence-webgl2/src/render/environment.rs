use crate::Device;
use coherence_base::model::{Environment, EnvironmentMap};

// need functions to write the environment map into the renderer?

impl Device {
    pub(crate) fn update_environment(&mut self, environment: &Environment) {
        if let Some(map) = &environment.map {
            self.envmap_texture
                .upload(map.width as usize, map.height as usize, &map.pixels);

            // compute the CDF data and load it into our buffers...
        }
    }
}
