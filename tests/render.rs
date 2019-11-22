use equinox::{Asset, WebDevice, WebScene};
use libflate::zlib::Decoder as ZlibDecoder;
use png::{BitDepth, ColorType, Decoder};
use serde::Deserialize;
use serde_json::json;
use std::io::Read;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

wasm_bindgen_test_configure!(run_in_browser);

struct AssetData {
    name: &'static str,
    data: &'static [u8],
}

const ASSET_DATA: &[AssetData] = &[AssetData {
    name: "assets/blue_grotto_4k.raw",
    data: include_bytes!("../assets/blue_grotto_4k.raw"),
}];

macro_rules! render_scene {
    ($name:ident, samples: $samples:expr) => {
        #[wasm_bindgen_test]
        fn $name() {
            let context = setup_context(setup_canvas());

            perform_test(
                &mut WebScene::new(),
                &mut setup_device(&context),
                &context,
                &TestData {
                    name: stringify!($name),
                    json: include_str!(concat!("scenes/", stringify!($name), ".json")),
                    png: include_bytes!(concat!("scenes/", stringify!($name), ".png")),
                    samples: $samples,
                },
            );
        }
    };
}

render_scene!(default_scene, samples: 265);
render_scene!(pink_glass, samples: 210);

struct TestData {
    name: &'static str,
    json: &'static str,
    png: &'static [u8],
    samples: usize,
}

fn perform_test(
    mut scene: &mut WebScene,
    device: &mut WebDevice,
    context: &WebGl2RenderingContext,
    test_data: &TestData,
) {
    load_scene(&mut scene, test_data.json);

    update_device(&context, device, &mut scene);

    for _ in 0..test_data.samples {
        device.refine().unwrap();
    }

    device.render().unwrap();

    let rendered_bytes = read_canvas_pixels(&context);
    let expected_bytes = read_png_pixels(test_data.png);

    assert_eq!(
        expected_bytes.len(),
        rendered_bytes.len(),
        "{}: rendered image byte size differs",
        test_data.name
    );

    if rendered_bytes != expected_bytes {
        panic!("{}: rendered image bytes differ", test_data.name);
    }
}

fn setup_canvas() -> HtmlCanvasElement {
    web_sys::window()
        .expect("window not found")
        .document()
        .expect("document not found")
        .create_element("canvas")
        .expect("failed to create canvas")
        .dyn_into()
        .expect("failed to create canvas")
}

fn setup_context(canvas: HtmlCanvasElement) -> WebGl2RenderingContext {
    let options: JsValue = JsValue::from_serde(&json!({
        "alpha": false,
        "depth": false,
        "stencil": false,
        "antialias": false,
    }))
    .unwrap();

    canvas
        .get_context_with_context_options("webgl2", &options)
        .expect("failed to create context")
        .expect("context is not supported")
        .dyn_into()
        .expect("failed to create context")
}

fn setup_device(context: &WebGl2RenderingContext) -> WebDevice {
    WebDevice::new(context).expect("failed to create device")
}

fn load_scene(scene: &mut WebScene, json: &str) {
    #[derive(Deserialize)]
    struct SceneJsonData {
        json: serde_json::Value,
        assets: Vec<Asset>,
    }

    let data: SceneJsonData = serde_json::from_str(json).expect("failed to parse scene json");

    scene
        .set_json(&JsValue::from_serde(&data.json).unwrap())
        .expect("failed to load scene from scene json data");

    for asset in &data.assets {
        scene.insert_asset(asset, &get_asset(asset));
    }
}

fn get_asset(name: &str) -> Vec<u8> {
    for asset_data in ASSET_DATA {
        if asset_data.name == name {
            let mut decompressed = vec![];

            ZlibDecoder::new(asset_data.data)
                .expect("failed to decompress asset")
                .read_to_end(&mut decompressed)
                .expect("failed to decompress asset");

            return decompressed;
        }
    }

    panic!("asset {} not found in test asset data", name);
}

fn update_device(context: &WebGl2RenderingContext, device: &mut WebDevice, scene: &mut WebScene) {
    device
        .update(scene)
        .expect("failed to update device with scene");

    let canvas = get_canvas(context);

    canvas.set_width(scene.raster_width());
    canvas.set_height(scene.raster_height());
}

fn read_canvas_pixels(context: &WebGl2RenderingContext) -> Vec<u8> {
    let canvas = get_canvas(context);

    let mut flipped_bytes = vec![0; (canvas.width() * canvas.height()) as usize * 4];

    context
        .read_pixels_with_opt_u8_array(
            0,
            0,
            canvas.width() as i32,
            canvas.height() as i32,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            Some(&mut flipped_bytes),
        )
        .expect("failed to read canvas pixels");

    let mut bytes = vec![0; (canvas.width() * canvas.height()) as usize * 4];

    for y in 0..canvas.height() {
        let target_row = canvas.height() - 1 - y;

        let row_length = canvas.width() * 4;
        let src_offset = row_length * y;
        let dst_offset = row_length * target_row;

        for i in 0..row_length {
            bytes[(src_offset + i) as usize] = flipped_bytes[(dst_offset + i) as usize];
        }
    }

    bytes
}

fn read_png_pixels(png: &[u8]) -> Vec<u8> {
    let (info, mut reader) = Decoder::new(png)
        .read_info()
        .expect("failed to parse png data");

    assert_eq!(info.color_type, ColorType::RGBA);
    assert_eq!(info.bit_depth, BitDepth::Eight);

    let mut bytes = vec![0u8; (info.width * info.height) as usize * 4];

    reader
        .next_frame(&mut bytes)
        .expect("failed to read png pixels");

    bytes
}

fn get_canvas(context: &WebGl2RenderingContext) -> HtmlCanvasElement {
    context
        .canvas()
        .unwrap()
        .dyn_into()
        .expect("failed to create context")
}
