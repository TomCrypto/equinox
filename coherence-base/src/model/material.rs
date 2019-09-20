use cgmath::Vector3;

#[derive(Default)]
pub struct Materials {
    pub list: Vec<Material>,
}

pub enum Material {
    Diffuse { color: Vector3<f32> },
    Specular,
    Emissive { strength: f32 },
}
