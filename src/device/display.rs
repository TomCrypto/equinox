use crate::Device;
use crate::Display;
use js_sys::Error;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(Debug, AsBytes, FromBytes)]
pub struct DisplayData {
    exposure: f32,
    saturation: f32,
    has_camera_response: u32,
    padding: f32,
    camera_response: [[f32; 4]; 11],
}

impl Device {
    pub(crate) fn update_display(&mut self, display: &Display) -> Result<(), Error> {
        let data: &mut DisplayData = self.allocator.allocate_one();

        data.exposure = (2.0f32).powf(display.exposure);
        data.saturation = display.saturation.max(0.0).min(1.0);

        if let Some(camera_response) = display.camera_response {
            for (index, camera_response) in camera_response.iter().enumerate() {
                data.camera_response[index][0] = camera_response[0];
                data.camera_response[index][1] = camera_response[1];
                data.camera_response[index][2] = camera_response[2];
            }

            data.has_camera_response = 1;
        } else {
            data.has_camera_response = 0;
        }

        self.display_buffer.write(data)
    }
}
