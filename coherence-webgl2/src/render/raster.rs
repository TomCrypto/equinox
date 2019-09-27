use crate::Device;
use coherence_base::Raster;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(AsBytes, FromBytes)]
pub struct RasterData {
    width: f32,
    height: f32,
    inv_width: f32,
    inv_height: f32,
}

impl Device {
    pub(crate) fn update_raster(&mut self, raster: &Raster) {
        let data = RasterData {
            width: raster.width.get() as f32,
            height: raster.height.get() as f32,
            inv_width: 1.0 / (raster.width.get() as f32),
            inv_height: 1.0 / (raster.height.get() as f32),
        };

        self.raster_buffer.write(&data);
    }
}
