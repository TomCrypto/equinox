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

    pub fn set_bvh_data(&mut self, data: &[u8]) {
        *self.scene.bvh_data = data.to_vec();
    }

    pub fn set_tri_data(&mut self, data: &[u8]) {
        *self.scene.tri_data = data.to_vec();
    }

    pub fn set_dimensions(&mut self, width: u32, height: u32) {
        self.scene.frame.width = NonZeroU32::new(width).unwrap();
        self.scene.frame.height = NonZeroU32::new(height).unwrap();
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.scene.frame.seed = seed;
    }

    pub fn add_model(&mut self, bvh: &[u8], triangles: &[u8]) -> usize {
        self.scene.models.push(Model {
            bvh: bvh.to_vec(),
            triangles: triangles.to_vec(),
        });

        self.scene.models.len() - 1
    }

    pub fn delete_model(&mut self, index: usize) {
        self.scene.models.remove(index);
    }
}
