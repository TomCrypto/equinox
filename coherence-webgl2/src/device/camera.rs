use crate::Device;
use cgmath::prelude::*;
use cgmath::Point3;
use coherence_base::{Aperture, Camera};
use itertools::iproduct;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Default, AsBytes, FromBytes)]
pub struct CameraData {
    origin_plane: [[f32; 4]; 4],
    target_plane: [[f32; 4]; 4],
    aperture_settings: [f32; 4],
}

impl Device {
    pub(crate) fn update_camera(&mut self, camera: &Camera) {
        let data: &mut CameraData = self.scratch.allocate_one();

        let fov_tan = camera.film_height / (2.0 * camera.focal_length);

        let mut xfm: cgmath::Matrix4<f32> = Transform::look_at(
            camera.position,
            camera.position + camera.direction,
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

        self.camera_buffer.write(data);
    }
}

fn aperture_settings(aperture: &Aperture) -> [f32; 4] {
    match aperture {
        Aperture::Point => [-1.0; 4],
        Aperture::Circle { .. } => [0.0, 0.0, 0.0, 0.0],
        Aperture::Ngon {
            sides, rotation, ..
        } => [1.0, *sides as f32, *rotation as f32, 1.0 / (*sides as f32)],
    }
}
