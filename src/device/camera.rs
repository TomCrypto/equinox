use crate::{ApertureShape, Camera, Device};
use cgmath::prelude::*;
use cgmath::{Matrix4, Point3, Vector3};
use js_sys::Error;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug, Default)]
pub struct CameraData {
    aperture_settings: [f32; 4],
    camera_transform: [[f32; 4]; 4],
    camera_settings: [f32; 4],
}

impl Device {
    pub(crate) fn update_camera(&mut self, camera: &Camera) -> Result<(), Error> {
        let mut data = CameraData::default();

        let position: Point3<f32> = camera.position.into();
        let mut direction: Vector3<f32> = camera.direction.into();
        let mut up_vector: Vector3<f32> = camera.up_vector.into();

        direction = direction.normalize();
        up_vector = up_vector.normalize();

        // Matrix4::look_at uses a right-handed coordinate system, which is wrong for
        // us. The easiest way to work around it is by negating the camera direction.

        let xfm: Matrix4<f32> = Transform::look_at(position, position - direction, up_vector);

        data.aperture_settings = aperture_settings(&camera.aperture);
        data.camera_transform = xfm.inverse_transform().unwrap().into();

        data.camera_settings[0] = camera.film_height / (2.0 * camera.focal_length);
        data.camera_settings[1] = camera.focal_distance;
        data.camera_settings[2] = camera.focal_curvature;
        data.camera_settings[3] = 0.0;

        self.camera_buffer.write(&data)
    }
}

fn aperture_settings(aperture: &ApertureShape) -> [f32; 4] {
    match aperture {
        ApertureShape::Point => [-1.0, 0.0, 0.0, 0.0],
        ApertureShape::Circle { radius } => [0.0, 0.0, 0.0, *radius],
        ApertureShape::Ngon {
            sides,
            rotation,
            radius,
        } => [1.0, *sides as f32, *rotation as f32, *radius],
    }
}
