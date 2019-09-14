use cgmath::prelude::*;
use cgmath::{Quaternion, Vector3};
use console_log;
use std::num::NonZeroU32;
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

use sigma_core::*;
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

    // TODO: return rendering stats later (in the form of Serde-serialized data I
    // guess)
    pub fn refine(&mut self) -> Result<(), JsValue> {
        self.device.update(&mut self.scene)?;
        self.device.refine();
        Ok(())
    }

    // TODO: return stats
    pub fn render(&mut self) -> Result<(), JsValue> {
        self.device.update(&mut self.scene)?;
        self.device.render();
        Ok(())
    }

    pub fn move_camera(&mut self, delta_x: f32, delta_y: f32, fov: f32) {
        self.scene.camera.fov = fov;
        self.scene.camera.rotate(delta_x, delta_y);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.scene.camera.zoom(factor);
    }

    pub fn add_object(&mut self, bvh: &[u8], tri: &[u8]) -> usize {
        self.scene.objects.list.push(Object {
            hierarchy: bvh.to_vec(),
            triangles: tri.to_vec(),
        });

        self.scene.objects.list.len() - 1
    }

    pub fn add_instance(&mut self, object: usize) {
        self.scene.instances.list.push(Instance {
            object,
            scale: 1.0,
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            translation: Vector3::new(0.0, 0.0, 0.0),
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

    pub fn instance_count(&mut self) -> usize {
        self.scene.instances.list.len()
    }

    pub fn remove_instance(&mut self, index: usize) {
        self.scene.instances.list.remove(index);
    }
}
