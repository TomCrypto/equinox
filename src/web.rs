use cgmath::prelude::*;
use cgmath::Vector3;
use js_sys::{Array, Error};
use serde::{de::DeserializeOwned, Serialize};
use std::num::NonZeroU32;
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

use crate::{Device, Scene};

use crate::{EnvironmentMap, Geometry, Instance, Material, Parameter};

/// WASM binding for a scene.
#[wasm_bindgen]
pub struct WebScene {
    scene: Scene,
}

#[wasm_bindgen]
impl WebScene {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WebScene, JsValue> {
        Ok(Self {
            scene: Scene::default(),
        })
    }

    pub fn reset_to_default(&mut self) {
        self.scene = Scene::default();
    }

    pub fn json(&self) -> Result<JsValue, JsValue> {
        as_json(&self.scene)
    }

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

        if self.scene.geometries != new_scene.geometries {
            self.scene.geometries = new_scene.geometries;
        }

        if self.scene.materials != new_scene.materials {
            self.scene.materials = new_scene.materials;
        }

        if self.scene.raster != new_scene.raster {
            self.scene.raster = new_scene.raster;
        }

        if self.scene.instances != new_scene.instances {
            self.scene.instances = new_scene.instances;
        }

        if self.scene.aperture != new_scene.aperture {
            self.scene.aperture = new_scene.aperture;
        }

        Ok(())
    }

    pub fn assets(&self) -> Array {
        self.scene
            .assets
            .keys()
            .map(|k| JsValue::from_str(k))
            .collect()
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

    pub fn set_envmap(&mut self, name: &str, width: u32, height: u32) {
        self.scene.environment.map = Some(EnvironmentMap {
            pixels: name.to_owned(),
            width,
            height,
        });
    }

    // TODO: bunch of getters and setters for the scene, dirtying as needed

    pub fn move_camera(&mut self, forward: f32, sideways: f32) {
        let sideways_vector = self
            .scene
            .camera
            .up_vector
            .cross(self.scene.camera.direction)
            .normalize();

        let direction = self.scene.camera.direction;

        self.scene.camera.position += forward * direction + sideways_vector * sideways;
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

    // TODO: remove eventually
    pub fn setup_test_scene(&mut self) {
        self.scene.camera.position.x = 0.0;
        self.scene.camera.position.y = 1.0;
        self.scene.camera.position.z = 5.0;

        self.scene.geometries.push(Geometry::Plane {
            width: Parameter::Constant { value: 30.0 },
            length: Parameter::Constant { value: 30.0 },
        });

        self.scene.geometries.push(Geometry::Translate {
            f: Box::new(Geometry::InfiniteRepetition {
                period: [
                    Parameter::Constant { value: 3.0 },
                    Parameter::Constant { value: 0.0 },
                    Parameter::Constant { value: 3.0 },
                ],
                f: Box::new(Geometry::UnitSphere),
            }),
            translation: [
                Parameter::Constant { value: 0.0 },
                Parameter::Constant { value: 1.01 },
                Parameter::Constant { value: 0.0 },
            ],
        });

        self.scene.geometries.push(Geometry::Union {
            children: vec![
                Geometry::Translate {
                    f: Box::new(Geometry::Scale {
                        f: Box::new(Geometry::UnitSphere),
                        factor: Parameter::Constant { value: 0.333 },
                    }),
                    translation: [
                        Parameter::Constant { value: 2.0 },
                        Parameter::Constant { value: 0.0 },
                        Parameter::Constant { value: 0.2 },
                    ],
                },
                Geometry::Translate {
                    f: Box::new(Geometry::Scale {
                        f: Box::new(Geometry::UnitSphere),
                        factor: Parameter::Constant { value: 0.333 },
                    }),
                    translation: [
                        Parameter::Constant { value: 2.0 },
                        Parameter::Constant { value: 0.0 },
                        Parameter::Constant { value: -0.2 },
                    ],
                },
            ],
        });

        // white lambertian
        self.scene.materials.push(Material::Lambertian {
            albedo: [0.9, 0.9, 0.9],
        });

        self.scene.materials.push(Material::Lambertian {
            albedo: [0.25, 0.25, 0.75],
        });

        /*self.scene.materials.push(Material::Phong {
            albedo: [0.9, 0.9, 0.9],
            shininess: 1024.0,
        });*/

        self.scene.materials.push(Material::IdealReflection {
            reflectance: [0.9, 0.9, 0.9],
        });

        self.scene.instances.push(Instance {
            geometry: 0,
            material: 0,
            geometry_values: vec![],
        });

        /*self.scene.instances.push(Instance {
            geometry: 1,
            material: 1,
            geometry_values: vec![],
            material_values: vec![0.8, 0.8, 0.8, 0.0],
        });*/

        self.scene.instances.push(Instance {
            geometry: 1,
            material: 2,
            geometry_values: vec![],
        });

        /*self.scene.instances.push(Instance {
            geometry: 1,
            material: 3,
            geometry_values: vec![],
            material_values: vec![0.8, 0.8, 0.8, 1.55],
        });*/

        /*self.scene.instances.push(Instance {
            geometry: 2,
            material: 1,
            geometry_values: vec![],
        });*/
    }
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

    pub fn context_lost(&mut self) {
        self.device.context_lost();
    }
}

const VERSION: &str = concat!("Equinox v", env!("CARGO_PKG_VERSION"), " (WebGL2)");

#[wasm_bindgen]
pub fn version() -> String {
    VERSION.to_owned()
}

#[wasm_bindgen]
pub fn initialize_logging() {
    console_error_panic_hook::set_once();
    let _ = console_log::init();
}

fn as_json<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    Ok(JsValue::from_serde(value).map_err(|err| Error::new(&err.to_string()))?)
}

fn from_json<T: DeserializeOwned>(json: &JsValue) -> Result<T, JsValue> {
    Ok(json
        .into_serde()
        .map_err(|err| Error::new(&err.to_string()))?)
}
