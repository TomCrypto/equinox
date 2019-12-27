//! The Equinox stochastic photon mapper, see the README for more information.

#![allow(clippy::too_many_arguments)]
#![allow(clippy::module_inception)]
#![forbid(unsafe_code, while_true)]

mod device {
    pub mod camera;
    pub mod device;
    pub mod display;
    pub mod environment;
    pub mod geometry;
    pub mod instance;
    pub mod integrator;
    pub mod lens_flare;
    pub mod material;
    pub mod raster;
}

mod engine {
    pub mod framebuffer;
    pub mod shader;
    pub mod texture;
    pub mod uniform_buffer;
    pub mod vertex_array;
}

mod scene {
    pub mod aperture;
    pub mod bounding_box;
    pub mod camera;
    pub mod dirty;
    pub mod display;
    pub mod environment;
    pub mod geometry;
    pub mod instance;
    pub mod integrator;
    pub mod material;
    pub mod metadata;
    pub mod raster;
    pub mod scene;
}

pub use device::{
    camera::*, device::*, display::*, environment::*, geometry::*, instance::*, integrator::*,
    lens_flare::*, material::*, raster::*,
};
pub use engine::{framebuffer::*, shader::*, texture::*, uniform_buffer::*, vertex_array::*};
pub use scene::{
    aperture::*, bounding_box::*, camera::*, dirty::*, display::*, environment::*, geometry::*,
    instance::*, integrator::*, material::*, metadata::*, raster::*, scene::*,
};

/// WebGL shaders from the `shader` directory.
///
/// This module is autogenerated by the crate build script which will handle all
/// GLSL preprocessing such as expanding #includes and adding file/line markers.
pub mod shader {
    include!(concat!(env!("OUT_DIR"), "/glsl_shaders.rs"));
}

use cgmath::{prelude::*, Basis3, Vector3};
use js_sys::{Array, Error, Function, Uint8Array};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::WebGl2RenderingContext;

/// WASM binding for a scene.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct WebScene {
    scene: Scene,
}

#[wasm_bindgen]
impl WebScene {
    /// Creates a new empty scene.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebScene {
        Self::default()
    }

    pub fn json(&self) -> Result<JsValue, JsValue> {
        as_json(&self.scene)
    }

    pub fn raster_width(&self) -> u32 {
        self.scene.raster.width
    }

    pub fn raster_height(&self) -> u32 {
        self.scene.raster.height
    }

    pub fn name(&self) -> String {
        self.scene.metadata.name.clone()
    }

    /// Reconfigures the scene using the provided scene JSON data.
    ///
    /// This method will attempt to dirty the least amount of scene data
    /// possible, so it won't necessarily always dirty the entire scene.
    pub fn set_json(&mut self, json: &JsValue) -> Result<(), JsValue> {
        self.scene.patch_from_other(from_json(json)?);

        Ok(())
    }

    /// Returns all assets which are referenced in this scene.
    ///
    /// All distinct assets will be returned in lexicographical order. Assets
    /// which are referenced in the scene but aren't used are still returned.
    pub fn assets(&self) -> Array {
        self.scene
            .assets()
            .into_iter()
            .map(ToOwned::to_owned)
            .map(JsValue::from)
            .collect()
    }

    pub fn set_raster_dimensions(&mut self, width: u32, height: u32) {
        if self.scene.raster.width != width {
            self.scene.raster.width = width;
        }

        if self.scene.raster.height != height {
            self.scene.raster.height = height;
        }
    }

    pub fn set_environment_rotation(&mut self, new_rotation: f32) {
        Dirty::modify(&mut self.scene.environment, |environment| {
            if let Environment::Map { rotation, .. } = environment {
                *rotation = new_rotation;
            }
        });
    }

    pub fn set_envmap(&mut self, name: &str) {
        Dirty::modify(&mut self.scene.environment_map, |environment_map| {
            *environment_map = Some(name.to_owned());
        });

        Dirty::modify(&mut self.scene.environment, |environment| {
            if let Environment::Map { .. } = environment {
                // do nothing; we're already in map mode
            } else {
                *environment = Environment::Map {
                    tint: [1.0; 3],
                    rotation: 0.0,
                };
            }
        });
    }

    /// Applies a camera-space translation to the camera position.
    pub fn move_camera(&mut self, dx: f32, dy: f32, dz: f32) {
        let mut direction: Vector3<f32> = self.scene.camera.direction.into();
        let mut up_vector: Vector3<f32> = self.scene.camera.up_vector.into();

        direction = direction.normalize();
        up_vector = up_vector.normalize();

        let xfm = Basis3::look_at(direction, up_vector).invert();
        let rotated_dir = xfm.rotate_vector([dx, dy, dz].into());

        self.scene.camera.position[0] += rotated_dir[0];
        self.scene.camera.position[1] += rotated_dir[1];
        self.scene.camera.position[2] += rotated_dir[2];
    }

    pub fn set_camera_direction(&mut self, x: f32, y: f32, z: f32) {
        if self.scene.camera.direction != [x, y, z] {
            self.scene.camera.direction = [x, y, z];
        }
    }
}

fn as_json<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    Ok(JsValue::from_serde(value).map_err(|e| Error::new(&e.to_string()))?)
}

fn from_json<T: DeserializeOwned>(json: &JsValue) -> Result<T, JsValue> {
    Ok(json.into_serde().map_err(|e| Error::new(&e.to_string()))?)
}

/// WASM binding for a device.
#[wasm_bindgen]
pub struct WebDevice {
    device: Device,
}

#[wasm_bindgen]
impl WebDevice {
    #[wasm_bindgen(constructor)]
    pub fn new(context: &WebGl2RenderingContext) -> Result<WebDevice, JsValue> {
        Ok(Self {
            device: Device::new(context)?,
        })
    }

    pub fn texture_compression(&mut self) -> Result<JsValue, JsValue> {
        Ok(JsValue::from_serde(&self.device.texture_compression()?).unwrap())
    }

    /// Returns whether updating the device with a scene may be time-consuming.
    pub fn is_expensive_update(&mut self, scene: &WebScene) -> Result<bool, JsValue> {
        Ok(self.device.is_update_expensive(&scene.scene)?)
    }

    /// Updates the device with a scene, returning true if an update occurred.
    pub fn update(&mut self, scene: &mut WebScene, assets: &Function) -> Result<bool, JsValue> {
        Ok(self.device.update(&mut scene.scene, |asset| {
            let asset_data = assets.call1(&JsValue::NULL, &JsValue::from(asset))?;

            if asset_data.is_null() || asset_data.is_undefined() {
                return Err(Error::new("failed to fetch asset"));
            }

            if let Some(buffer) = asset_data.dyn_ref::<Uint8Array>() {
                Ok(buffer.to_vec())
            } else {
                Err(Error::new("asset callback did not return Uint8Array"))
            }
        })?)
    }

    /// Refines the render using the integrator.
    pub fn refine(&mut self) -> Result<(), JsValue> {
        Ok(self.device.refine()?)
    }

    /// Presents the current integrator data.
    pub fn present(&mut self) -> Result<(), JsValue> {
        Ok(self.device.present()?)
    }

    /// Returns the number of photons traced by the SPPM integrator.
    pub fn sppm_photons(&self) -> f64 {
        self.device.state.photon_count as f64
    }

    /// Returns the number of passes performed by the SPPM integrator.
    pub fn sppm_passes(&self) -> u32 {
        self.device.state.current_pass
    }

    /// Indicates to the device that its WebGL context has been lost.
    pub fn context_lost(&mut self) {
        self.device.context_lost();
    }
}

#[allow(dead_code)]
mod build_metadata {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Returns a version string for the WASM module.
#[wasm_bindgen]
pub fn version() -> String {
    format!(
        "Equinox v{} ({}) built with {}",
        build_metadata::PKG_VERSION,
        build_metadata::GIT_VERSION.unwrap(),
        build_metadata::RUSTC_VERSION,
    )
}

/// Returns licensing information for the WASM module.
#[wasm_bindgen]
pub fn licensing() -> String {
    lies::licenses_text!().to_owned()
}

/// Configures browser logging functionality.
///
/// This initialization function is always safe to call more than once, so it
/// can be called safely every time the UI is hot-reloaded without panicking.
#[wasm_bindgen]
pub fn initialize_logging() {
    console_error_panic_hook::set_once();
    let _ = console_log::init();
}
