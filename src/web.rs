use crate::{Device, Scene};
use cgmath::prelude::*;
use cgmath::{Basis3, Vector3};
use js_sys::{Array, Error};
use serde::{de::DeserializeOwned, Serialize};
use std::num::NonZeroU32;
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

use crate::{ApertureShape, EnvironmentMap, Geometry, Instance, Material, Parameter};

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
        self.scene.raster.width.get()
    }

    pub fn raster_height(&self) -> u32 {
        self.scene.raster.height.get()
    }

    /// Reconfigures the scene using the provided scene JSON data.
    ///
    /// This method will attempt to dirty the least amount of scene data
    /// possible, so it won't necessarily always dirty the entire scene.
    pub fn set_json(&mut self, json: &JsValue) -> Result<(), JsValue> {
        let new_scene: Scene = from_json(json)?;

        if self.scene.camera != new_scene.camera {
            self.scene.camera = new_scene.camera;
        }

        if self.scene.display != new_scene.display {
            self.scene.display = new_scene.display;
        }

        if self.scene.environment != new_scene.environment {
            self.scene.environment = new_scene.environment;
        }

        if self.scene.geometry_list != new_scene.geometry_list {
            self.scene.geometry_list = new_scene.geometry_list;
        }

        if self.scene.material_list != new_scene.material_list {
            self.scene.material_list = new_scene.material_list;
        }

        if self.scene.raster != new_scene.raster {
            self.scene.raster = new_scene.raster;
        }

        if self.scene.instance_list != new_scene.instance_list {
            self.scene.instance_list = new_scene.instance_list;
        }

        if self.scene.aperture != new_scene.aperture {
            self.scene.aperture = new_scene.aperture;
        }

        Ok(())
    }

    /// Returns the list of all assets in the scene as a JS string array.
    pub fn assets(&self) -> Array {
        self.scene.assets.keys().map(JsValue::from).collect()
    }

    /// Deletes a geometry by index safely without corrupting the scene.
    ///
    /// Any instance using this geometry will be deleted, and any instance
    /// using a geometry after the deleted one will be adjusted as needed.
    pub fn delete_geometry(&mut self, index: usize) -> bool {
        if self.scene.geometry_list.len() < index {
            self.scene.geometry_list.remove(index);
        } else {
            return false;
        }

        self.scene.instance_list.retain(|i| i.geometry != index);

        for instance in self.scene.instance_list.iter_mut() {
            if instance.geometry > index {
                instance.geometry -= 1;
            }
        }

        true
    }

    /// Deletes a material by index safely without corrupting the scene.
    ///
    /// Any instance using this material will be deleted, and any instance
    /// using a material after the deleted one will be adjusted as needed.
    pub fn delete_material(&mut self, index: usize) -> bool {
        if self.scene.material_list.len() >= index {
            self.scene.material_list.remove(index);
        } else {
            return false;
        }

        self.scene.instance_list.retain(|i| i.material != index);

        for instance in self.scene.instance_list.iter_mut() {
            if instance.material > index {
                instance.material -= 1;
            }
        }

        true
    }

    pub fn set_raster_dimensions(&mut self, width: u32, height: u32) {
        if self.scene.raster.width.get() != width {
            self.scene.raster.width = NonZeroU32::new(width).unwrap();
        }

        if self.scene.raster.height.get() != height {
            self.scene.raster.height = NonZeroU32::new(height).unwrap();
        }
    }

    pub fn insert_asset(&mut self, name: &str, data: &[u8]) {
        self.scene.assets.insert(name.to_owned(), data.to_vec());
    }

    pub fn remove_asset(&mut self, name: &str) {
        self.scene.assets.remove(name);
    }

    pub fn set_envmap(&mut self, name: &str) {
        self.scene.environment.map = Some(EnvironmentMap {
            pixels: name.to_owned(),
            rotation: 0.0,
        });
    }

    /// Applies a camera-space translation to the camera position.
    pub fn move_camera(&mut self, dx: f32, dy: f32, dz: f32) {
        let xfm =
            Basis3::look_at(self.scene.camera.direction, self.scene.camera.up_vector).invert();

        self.scene.camera.position += xfm.rotate_vector(Vector3::new(dx, dy, dz));
    }

    pub fn set_camera_direction(&mut self, x: f32, y: f32, z: f32) {
        self.scene.camera.direction = Vector3::new(x, y, z);
    }

    pub fn orient_camera(&mut self, phi: f32, theta: f32) {
        let new_vector = Vector3::new(
            phi.cos() * theta.sin(),
            theta.cos(),
            phi.sin() * theta.sin(),
        );

        let change = cgmath::Quaternion::between_vectors(Vector3::new(0.0, 1.0, 0.0), new_vector);

        self.scene.camera.direction = change.rotate_vector(self.scene.camera.direction);
    }

    /// Sets the scene to a default scene.
    pub fn set_default_scene(&mut self) {
        self.scene = Scene::default();

        // Set up an "interesting" default scene below. We do this here because we
        // have proper types whereas doing it in the front-end would require JSON.

        self.scene.geometry_list.push(Geometry::Plane {
            width: Parameter::Constant { value: 1.0 },
            length: Parameter::Constant { value: 1.0 },
        });

        self.scene.geometry_list.push(Geometry::Translate {
            translation: [
                Parameter::Constant { value: 0.0 },
                Parameter::Constant { value: 0.51 },
                Parameter::Constant { value: 0.0 },
            ],
            f: Box::new(Geometry::Scale {
                factor: Parameter::Constant { value: 0.5 },
                f: Box::new(Geometry::Sphere {
                    radius: Parameter::Constant { value: 1.0 },
                }),
            }),
        });

        self.scene.geometry_list.push(Geometry::Onion {
            thickness: Parameter::Constant { value: 0.1 },
            f: Box::new(Geometry::Translate {
                translation: [
                    Parameter::Constant { value: 0.6 },
                    Parameter::Constant { value: 0.151 },
                    Parameter::Constant { value: 0.55 },
                ],
                f: Box::new(Geometry::Round {
                    radius: Parameter::Constant { value: 0.05 },
                    f: Box::new(Geometry::Scale {
                        factor: Parameter::Constant { value: 0.1 },
                        f: Box::new(Geometry::Cuboid {
                            dimensions: [
                                Parameter::Constant { value: 1.0 },
                                Parameter::Constant { value: 1.0 },
                                Parameter::Constant { value: 1.0 },
                            ],
                        }),
                    }),
                }),
            }),
        });

        self.scene.geometry_list.push(Geometry::Translate {
            translation: [
                Parameter::Constant { value: -0.6 },
                Parameter::Constant { value: 0.151 },
                Parameter::Constant { value: -0.75 },
            ],
            f: Box::new(Geometry::Scale {
                factor: Parameter::Constant { value: 0.15 },
                f: Box::new(Geometry::Sphere {
                    radius: Parameter::Constant { value: 1.0 },
                }),
            }),
        });

        self.scene.material_list.push(Material::Phong {
            albedo: [0.75, 0.75, 0.75],
            shininess: 900.0,
        });

        self.scene.material_list.push(Material::IdealReflection {
            reflectance: [0.9, 0.9, 0.9],
        });

        self.scene.material_list.push(Material::OrenNayar {
            albedo: [0.1, 0.7, 0.7],
            roughness: 0.4,
        });

        /*self.scene.material_list.push(Material::IdealRefraction {
            transmittance: [0.6, 0.7, 1.0],
            refractive_index: 1.65,
        });*/

        self.scene.material_list.push(Material::Dielectric {
            internal_refractive_index: 1.65,
            external_refractive_index: 1.0,
            internal_extinction_coefficient: [1e-7, 1e-7, 1e-7],
            external_extinction_coefficient: [0.0, 0.0, 0.0],
            base_color: [1.0, 1.0, 1.0],
        });

        self.scene.material_list.push(Material::Lambertian {
            albedo: [0.9, 0.9, 0.9],
        });

        self.scene.instance_list.push(Instance {
            geometry: 0,
            material: 0,
            parameters: vec![],
        });

        self.scene.instance_list.push(Instance {
            geometry: 1,
            material: 2,
            parameters: vec![],
        });

        self.scene.instance_list.push(Instance {
            geometry: 2,
            material: 2,
            parameters: vec![],
        });
        /*

        self.scene.instance_list.push(Instance {
            geometry: 3,
            material: 3,
            parameters: vec![],
        });*/

        self.scene.camera.position.x = 0.0;
        self.scene.camera.position.y = 0.5;
        self.scene.camera.position.z = 3.5;

        self.scene.camera.direction.x = 0.0;
        self.scene.camera.direction.y = 0.0;
        self.scene.camera.direction.z = -1.0;

        self.scene.camera.aperture = ApertureShape::Circle { radius: 0.0 };

        self.scene.camera.focal_distance = 3.0;
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

    pub fn refine(&mut self) {
        self.device.refine();
    }

    pub fn render(&mut self) {
        self.device.render();
    }

    pub fn sample_count(&self) -> u32 {
        self.device.state.frame
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
