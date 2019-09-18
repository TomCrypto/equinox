use crate::device::ToDevice;
use crate::model::Raster;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(AsBytes, FromBytes)]
pub struct RasterData {
    width: f32,
    height: f32,
    inv_width: f32,
    inv_height: f32,
}

impl ToDevice<RasterData> for Raster {
    fn to_device(&self, data: &mut RasterData) {
        data.width = self.width.get() as f32;
        data.height = self.height.get() as f32;
        data.inv_width = 1.0 / data.width;
        data.inv_height = 1.0 / data.height;
    }
}
