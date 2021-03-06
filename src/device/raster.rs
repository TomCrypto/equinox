use crate::{Device, Raster};
use js_sys::Error;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug, Default)]
pub struct RasterData {
    width: f32,
    height: f32,
    inv_width: f32,
    inv_height: f32,
}

impl Device {
    pub(crate) fn update_raster(&mut self, raster: &Raster) -> Result<(), Error> {
        let mut data = RasterData::default();

        data.width = raster.width as f32;
        data.height = raster.height as f32;
        data.inv_width = 1.0 / data.width;
        data.inv_height = 1.0 / data.height;

        self.raster_buffer.write(&data)
    }
}
