use crate::Device;
use coherence_base::Display;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(AsBytes, FromBytes)]
pub struct DisplayData {
    exposure: f32,
    saturation: f32,
    has_camera_response: u32,
    padding: f32,
    camera_response: [[f32; 4]; 11],
}

impl Device {
    pub(crate) fn update_display(&mut self, display: &Display) {
        let data: &mut DisplayData = self.scratch.allocate_one();

        data.exposure = (2.0f32).powf(display.exposure);
        data.saturation = display.saturation.max(0.0).min(1.0);

        if let Some(camera_response) = display.camera_response {
            for i in 0..11 {
                data.camera_response[i][0] = camera_response[i][0];
                data.camera_response[i][1] = camera_response[i][1];
                data.camera_response[i][2] = camera_response[i][2];
            }

            data.has_camera_response = 1;
        } else {
            data.has_camera_response = 0;
        }

        self.display_buffer.write(data);
    }
}
