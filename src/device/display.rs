use crate::{Device, Display};
use js_sys::Error;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug, Default)]
pub struct DisplayData {
    exposure: f32,
    saturation: f32,
    padding: [f32; 2],
}

impl Device {
    pub(crate) fn update_display(&mut self, display: &Display) -> Result<(), Error> {
        let mut data = DisplayData::default();

        data.exposure = (2.0f32).powf(display.exposure);
        data.saturation = display.saturation.max(0.0).min(1.0);

        self.display_buffer.write(&data)
    }
}
