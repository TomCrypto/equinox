#[allow(unused_imports)]
use log::{debug, info, warn};

use cgmath::prelude::*;
use cgmath::{Point3, Quaternion, Vector3};
use console_log;
use std::num::NonZeroU32;
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

use coherence_base::{model::*, *};
use coherence_webgl2::*;

#[wasm_bindgen]
pub struct WasmRunner {
    device: Device,
    scene: Scene,

    render_stats: Option<RenderStatistics>,
    refine_stats: Option<RefineStatistics>,
}

#[wasm_bindgen]
impl WasmRunner {
    #[wasm_bindgen(constructor)]
    pub fn new(context: &WebGl2RenderingContext) -> Result<WasmRunner, JsValue> {
        console_log::init().unwrap();
        console_error_panic_hook::set_once();

        Ok(Self {
            device: Device::new(context)?,
            scene: Scene::new(),
            render_stats: None,
            refine_stats: None,
        })
    }

    pub fn context_lost(&mut self) {
        self.device.context_lost();
    }

    pub fn context(&self) -> WebGl2RenderingContext {
        self.device.gl.clone()
    }

    // functions to do stuff... e.g. "rotate camera", "zoom in", "switch
    // perspective", "change material", etc... these are all "actions"
    // propagated to the scene

    pub fn update(&mut self) -> Result<bool, JsValue> {
        Ok(self.device.update(&mut self.scene)?)
    }

    pub fn refine(&mut self) {
        self.refine_stats = self.device.refine();
    }

    pub fn render(&mut self) {
        self.render_stats = self.device.render();
    }

    pub fn get_refine_frame_time(&self) -> Option<f32> {
        Some(self.refine_stats?.frame_time_us)
    }

    pub fn get_render_frame_time(&self) -> Option<f32> {
        Some(self.render_stats?.frame_time_us)
    }

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

    pub fn set_camera_position(&mut self, x: f32, y: f32, z: f32) {
        self.scene.camera.position = Point3::new(x, y, z);
    }

    pub fn set_camera_direction(&mut self, x: f32, y: f32, z: f32) {
        self.scene.camera.direction = Vector3::new(x, y, z);
    }

    pub fn set_camera_aperture(&mut self, radius: f32) {
        self.scene.camera.aperture = Aperture::Ngon {
            radius,
            sides: 8,
            rotation: 0.0,
        };
    }

    /*pub fn add_object(
        &mut self,
        bvh: &[u8],
        tri: &[u8],
        positions: &[u8],
        normal_tangent_uv: &[u8],
        materials: usize,
        bminx: f32,
        bminy: f32,
        bminz: f32,
        bmaxx: f32,
        bmaxy: f32,
        bmaxz: f32,
    ) -> usize {
        self.scene.objects.list.push(Object {
            hierarchy: bvh.to_vec(),
            triangles: tri.to_vec(),
            positions: positions.to_vec(),
            normal_tangent_uv: normal_tangent_uv.to_vec(),
            materials,
            bbox: BoundingBox {
                min: [bminx, bminy, bminz].into(),
                max: [bmaxx, bmaxy, bmaxz].into(),
            },
        });

        self.scene.objects.list.len() - 1
    }

    pub fn add_material(&mut self, kind: u32, r: f32, g: f32, b: f32) -> usize {
        if kind == 0 {
            self.scene.materials.list.push(Material::Diffuse {
                color: Vector3::new(r, g, b),
            });
        } else if kind == 1 {
            self.scene.materials.list.push(Material::Specular);
        } else if kind == 2 {
            self.scene
                .materials
                .list
                .push(Material::Emissive { strength: r })
        } else {
            panic!("bad kind")
        }

        self.scene.materials.list.len() - 1
    }

    pub fn add_instance(
        &mut self,
        object: usize,
        x: f32,
        y: f32,
        z: f32,
        scale: f32,
        materials: &[usize],
    ) {
        self.scene.instances.list.push(Instance {
            object,
            scale,
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            translation: Vector3::new(x, y, z),
            materials: materials.to_vec(),
        })
    }

    pub fn move_instance_up(&mut self, index: usize, amount: f32) {
        self.scene.instances.list[index].translation += Vector3::new(0.0, amount, 0.0);
    }*/

    pub fn add_other_object(&mut self) -> usize {
        // elongated cube

        self.scene.objects.list.push(Geometry::Translate {
            translation: [
                Parameter::Constant(1.5),
                Parameter::Constant(0.0),
                Parameter::Constant(0.0),
            ],
            f: Box::new(Geometry::Scale {
                factor: Parameter::Constant(0.5),
                f: Box::new(Geometry::UnitCube),
            }),
        });

        self.scene.objects.list.len() - 1
    }

    pub fn add_object(&mut self) -> usize {
        // for now, just add a sphere
        self.scene.objects.list.push(Geometry::Union {
            children: vec![
                Box::new(Geometry::Translate {
                    translation: [
                        Parameter::Symbolic(1),
                        Parameter::Symbolic(2),
                        Parameter::Symbolic(3),
                    ],
                    f: Box::new(Geometry::Scale {
                        factor: Parameter::Symbolic(0),
                        f: Box::new(Geometry::Round {
                            f: Box::new(Geometry::UnitCube),
                            radius: Parameter::Constant(0.125),
                        }),
                    }),
                }),
                Box::new(Geometry::Translate {
                    translation: [
                        Parameter::Symbolic(5),
                        Parameter::Symbolic(6),
                        Parameter::Symbolic(7),
                    ],
                    f: Box::new(Geometry::Scale {
                        factor: Parameter::Symbolic(4),
                        f: Box::new(Geometry::UnitSphere),
                    }),
                }),
            ],
        });

        self.scene.objects.list.len() - 1
    }

    pub fn add_instance(
        &mut self,
        geometry: usize,
        material: usize,
        parameters: &[f32],
        materials: &[f32],
    ) {
        self.scene.instances.list.push(Instance {
            geometry,
            material,
            geometry_values: parameters.to_vec(),
            material_values: materials.to_vec(),
        });
    }

    pub fn set_dimensions(&mut self, width: u32, height: u32) {
        self.scene.raster.width = NonZeroU32::new(width).unwrap();
        self.scene.raster.height = NonZeroU32::new(height).unwrap();
        self.scene.raster.filter = RasterFilter::BlackmanHarris;
    }

    pub fn set_focal_distance(&mut self, value: f32) {
        self.scene.camera.focal_distance = value;
    }

    pub fn set_focal_length(&mut self, value: f32) {
        self.scene.camera.focal_length = value;
    }

    pub fn instance_count(&mut self) -> usize {
        self.scene.instances.list.len()
    }

    pub fn remove_instance(&mut self, index: usize) {
        self.scene.instances.list.remove(index);
    }
}
