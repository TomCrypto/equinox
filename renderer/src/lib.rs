#[allow(unused_imports)]
use log::{debug, info, warn};

use cgmath::prelude::*;
use cgmath::{Quaternion, Vector3};
use console_log;
use std::num::NonZeroU32;
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

use sigma_core::{model::*, *};
use sigma_webgl2::*;

#[wasm_bindgen]
pub struct WasmRunner {
    device: Device,
    scene: Scene,
}

#[wasm_bindgen]
impl WasmRunner {
    #[wasm_bindgen(constructor)]
    pub fn new(context: WebGl2RenderingContext) -> Result<WasmRunner, JsValue> {
        console_log::init().unwrap();
        console_error_panic_hook::set_once();

        Ok(Self {
            device: Device::new(context)?,
            scene: Scene::new(),
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

    pub fn update(&mut self) -> Result<(), JsValue> {
        Ok(self.device.update(&mut self.scene)?)
    }

    // TODO: return rendering stats later (in the form of Serde-serialized data I
    // guess)
    pub fn refine(&mut self) {
        self.device.refine();
    }

    // TODO: return stats
    pub fn render(&mut self) {
        self.device.render();
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

    pub fn set_camera_direction(&mut self, x: f32, y: f32, z: f32) {
        self.scene.camera.direction = Vector3::new(x, y, z);
    }

    pub fn set_camera_aperture(&mut self, radius: f32) {
        self.scene.camera.aperture = Aperture::Ngon {
            radius,
            sides: 6,
            rotation: 0.0,
        };
    }

    pub fn add_object(
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

    pub fn add_instance(&mut self, object: usize, x: f32, y: f32, z: f32) {
        self.scene.instances.list.push(Instance {
            object,
            scale: 1.0,
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            translation: Vector3::new(x, y, z),
        })
    }

    pub fn move_instance_up(&mut self, index: usize, amount: f32) {
        self.scene.instances.list[index].translation += Vector3::new(0.0, amount, 0.0);
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
