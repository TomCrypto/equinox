pub struct EnvironmentMap {
    pub pixels: Vec<f32>,
    pub width: u32,
    pub height: u32,
    // TODO: assume equirectangular projection for now (most common by far)
}

#[derive(Default)]
pub struct Environment {
    pub map: Option<EnvironmentMap>,
    pub multiplier: [f32; 3],
}
