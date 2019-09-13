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
        self.device.context.clone()
    }

    // functions to do stuff... e.g. "rotate camera", "zoom in", "switch
    // perspective", "change material", etc... these are all "actions"
    // propagated to the scene

    // TODO: return rendering stats later (in the form of Serde-serialized data I
    // guess)
    pub fn refine(&mut self) {
        self.device.update(&mut self.scene);
        self.device.refine();
    }

    // TODO: return stats
    pub fn render(&mut self) {
        self.device.update(&mut self.scene);
        self.device.render();
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

    pub fn set_dimensions(&mut self, width: i32, height: i32) {
        *self.scene.dimensions = (width, height);
    }
}
