use crate::device::ToDevice;
use crate::model::{Aperture, Camera};
use cgmath::prelude::*;
use cgmath::Point3;
use itertools::iproduct;
use zerocopy::{AsBytes, FromBytes};

impl Aperture {
    fn settings(&self) -> [f32; 4] {
        match self {
            Self::Point => [-1.0; 4],
            Self::Circle { .. } => [0.0, 0.0, 0.0, 0.0],
            Self::Ngon {
                sides, rotation, ..
            } => [1.0, *sides as f32, *rotation as f32, 1.0 / (*sides as f32)],
        }
    }
}

#[repr(C)]
#[derive(Default, AsBytes, FromBytes)]
pub struct CameraData {
    origin_plane: [[f32; 4]; 4],
    target_plane: [[f32; 4]; 4],
    aperture_settings: [f32; 4],
}

impl ToDevice<CameraData> for Camera {
    fn to_device(&self, data: &mut CameraData) {
        let fov_tan = self.film_height / (2.0 * self.focal_length);

        let mut xfm: cgmath::Matrix4<f32> = Transform::look_at(
            self.position,
            self.position + self.direction,
            self.up_vector,
        );

        xfm = xfm.inverse_transform().unwrap();

        for (&x, &y) in iproduct!(&[-1i32, 1i32], &[-1i32, 1i32]) {
            let plane_index = (y + 1 + (x + 1) / 2) as usize;

            let origin = xfm.transform_point(Point3::new(
                (x as f32) * self.aperture.radius(),
                (y as f32) * self.aperture.radius(),
                0.0,
            ));

            let target = xfm.transform_point(Point3::new(
                (x as f32) * fov_tan * self.focal_distance,
                (y as f32) * fov_tan * self.focal_distance,
                self.focal_distance,
            ));

            data.origin_plane[plane_index] = [origin.x, origin.y, origin.z, 1.0];
            data.target_plane[plane_index] = [target.x, target.y, target.z, 1.0];
        }

        data.aperture_settings = self.aperture.settings();
    }

    fn requested_count(&self) -> usize {
        1
    }
}
