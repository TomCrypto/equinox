use crate::Device;
use crate::{ApertureShape, Camera};
use cgmath::prelude::*;
use cgmath::{Matrix4, Point3};
use itertools::iproduct;
use js_sys::Error;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(Debug, Default, AsBytes, FromBytes)]
pub struct CameraData {
    origin_plane: [[f32; 4]; 4],
    target_plane: [[f32; 4]; 4],
    aperture_settings: [f32; 4],
}

impl Device {
    pub(crate) fn update_camera(&mut self, camera: &Camera) -> Result<(), Error> {
        let data: &mut CameraData = self.allocator.allocate_one();

        let fov_tan = camera.film_height / (2.0 * camera.focal_length);

        // Matrix4::look_at uses a right-handed coordinate system, which is wrong for
        // us. The easiest way to work around is to just negate the camera direction.

        let mut xfm: Matrix4<f32> = Transform::look_at(
            camera.position,
            camera.position - camera.direction,
            camera.up_vector,
        );

        xfm = xfm.inverse_transform().unwrap();

        for (&x, &y) in iproduct!(&[-1i32, 1i32], &[-1i32, 1i32]) {
            let plane_index = (y + 1 + (x + 1) / 2) as usize;

            let origin = xfm.transform_point(Point3::new(
                (x as f32) * camera.aperture.radius(),
                (y as f32) * camera.aperture.radius(),
                0.0,
            ));

            let target = xfm.transform_point(Point3::new(
                (x as f32) * fov_tan * camera.focal_distance,
                (y as f32) * fov_tan * camera.focal_distance,
                camera.focal_distance,
            ));

            data.origin_plane[plane_index] = [origin.x, origin.y, origin.z, 1.0];
            data.target_plane[plane_index] = [target.x, target.y, target.z, 1.0];
        }

        data.aperture_settings = aperture_settings(&camera.aperture);

        self.camera_buffer.write(data)
    }
}

fn aperture_settings(aperture: &ApertureShape) -> [f32; 4] {
    match aperture {
        ApertureShape::Point => [-1.0; 4],
        ApertureShape::Circle { .. } => [0.0, 0.0, 0.0, 0.0],
        ApertureShape::Ngon {
            sides, rotation, ..
        } => [1.0, *sides as f32, *rotation as f32, 1.0 / (*sides as f32)],
    }
}
