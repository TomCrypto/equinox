use crate::{Device, Dirty, Scene};
use cgmath::prelude::*;
use cgmath::Basis3;
use js_sys::{Array, Error};
use maplit::btreemap;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

use crate::{ApertureShape, Environment, Geometry, Instance, Material, Parameter};

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

    /// Reconfigures the scene using the provided scene JSON data.
    ///
    /// This method will attempt to dirty the least amount of scene data
    /// possible, so it won't necessarily always dirty the entire scene.
    pub fn set_json(&mut self, json: &JsValue) -> Result<(), JsValue> {
        self.scene.patch_from_other(from_json(json)?);

        Ok(())
    }

    /// Returns the list of all assets in the scene as a JS string array.
    pub fn assets(&self) -> Array {
        self.scene.assets.keys().map(JsValue::from).collect()
    }

    pub fn set_raster_dimensions(&mut self, width: u32, height: u32) {
        Dirty::modify(&mut self.scene.raster, |raster| {
            raster.width = width;
            raster.height = height;
        });
    }

    pub fn insert_asset(&mut self, name: &str, data: &[u8]) {
        self.scene.assets.insert(name.to_owned(), data.to_vec());
    }

    pub fn remove_asset(&mut self, name: &str) {
        self.scene.assets.remove(name);
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
        let xfm =
            Basis3::look_at(self.scene.camera.direction, self.scene.camera.up_vector).invert();

        Dirty::modify(&mut self.scene.camera, |camera| {
            camera.position += xfm.rotate_vector([dx, dy, dz].into());
        });
    }

    pub fn set_camera_direction(&mut self, x: f32, y: f32, z: f32) {
        Dirty::modify(&mut self.scene.camera, |camera| {
            camera.direction = [x, y, z].into();
        });
    }

    /// Sets the scene to a default scene.
    pub fn set_default_scene(&mut self) {
        self.scene = Scene::default();

        // Set up an "interesting" default scene below. We do this here because we
        // have proper types whereas doing it in the front-end would require JSON.

        self.scene.geometry_list.insert(
            "ground".to_owned(),
            Geometry::Cuboid {
                dimensions: [
                    Parameter::Constant(3.0),
                    Parameter::Constant(0.01),
                    Parameter::Constant(3.0),
                ],
            },
        );

        self.scene.geometry_list.insert(
            "sphere".to_owned(),
            Geometry::Translate {
                translation: [
                    Parameter::Symbolic("x".to_owned()),
                    Parameter::Symbolic("y".to_owned()),
                    Parameter::Symbolic("z".to_owned()),
                ],
                f: Box::new(Geometry::Sphere {
                    radius: Parameter::Constant(0.799),
                }),
            },
        );

        self.scene.material_list.insert(
            "phong".to_owned(),
            Material::Phong {
                albedo: [0.9, 0.9, 0.9],
                shininess: 20.0,
            },
        );

        self.scene.material_list.insert(
            "lambertian".to_owned(),
            Material::Lambertian {
                albedo: [0.9, 0.6, 0.2],
            },
        );

        self.scene.material_list.insert(
            "diffuse1".to_owned(),
            Material::OrenNayar {
                albedo: [0.1, 0.7, 0.7],
                roughness: 1.0,
            },
        );

        self.scene.material_list.insert(
            "phong2".to_owned(),
            Material::Phong {
                albedo: [0.3, 0.9, 0.7],
                shininess: 700.0,
            },
        );

        self.scene.material_list.insert(
            "glass1".to_owned(),
            Material::Dielectric {
                internal_refractive_index: 2.2,
                external_refractive_index: 1.0,
                internal_extinction_coefficient: [1.0, 1.0, 1.0],
                external_extinction_coefficient: [0.0, 0.0, 0.0],
                base_color: [0.560, 0.570, 0.580],
            },
        );

        self.scene.material_list.insert(
            "glass2".to_owned(),
            Material::IdealRefraction {
                transmittance: [0.0, 1.0, 0.0],
                refractive_index: 1.3,
            },
        );

        self.scene.instance_list.insert(
            "ground".to_owned(),
            Instance {
                geometry: "ground".to_owned(),
                material: "lambertian".to_owned(),
                parameters: btreemap! {},
                photon_receiver: true,
                sample_explicit: true,
                visible: true,
            },
        );

        self.scene.instance_list.insert("glass-sphere".to_owned(), Instance {
            geometry: "sphere".to_owned(),
            material: "glass2".to_owned(),
            parameters: btreemap! {"x".to_owned() => 0.0, "y".to_owned() => 0.81, "z".to_owned() => 0.0 },
            photon_receiver: true,
            sample_explicit: true,
            visible: true,
        });

        self.scene.instance_list.insert("diffuse-sphere".to_owned(), Instance {
            geometry: "sphere".to_owned(),
            material: "diffuse1".to_owned(),
            parameters: btreemap! { "x".to_owned() => -2.0, "y".to_owned() => 0.81, "z".to_owned() => 0.0 },
            photon_receiver: true,
            sample_explicit: true,
            visible: true,
        });

        self.scene.camera.position.x = 0.0;
        self.scene.camera.position.y = 7.5;
        self.scene.camera.position.z = 14.2;

        self.scene.camera.direction.x = 0.0;
        self.scene.camera.direction.y = -0.5;
        self.scene.camera.direction.z = -0.85;

        self.scene.camera.aperture = ApertureShape::Circle { radius: 0.0 };

        self.scene.camera.focal_distance = 15.44;
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
#[derive(Debug)]
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

    /// Updates the device with a scene, returning true if an update occurred.
    pub fn update(&mut self, scene: &mut WebScene) -> Result<bool, JsValue> {
        Ok(self.device.update(&mut scene.scene)?)
    }

    /// Refines the render using the integrator.
    pub fn refine(&mut self) -> Result<(), JsValue> {
        Ok(self.device.refine()?)
    }

    /// Renders the current integrator data.
    pub fn render(&mut self) -> Result<(), JsValue> {
        Ok(self.device.render()?)
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

/// Returns a version string for the WASM module.
#[wasm_bindgen]
pub fn version() -> String {
    concat!("Equinox v", env!("CARGO_PKG_VERSION"), " (WebGL2)").to_owned()
}

/// Configures browser logging functionality.
///
/// This function is safe to call more than once and will do nothing should it
/// be called more than once; this lets it co-exist nicely with hot reloaders.
#[wasm_bindgen]
pub fn initialize_logging() {
    console_error_panic_hook::set_once();
    let _ = console_log::init();
}
