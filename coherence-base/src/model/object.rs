use crate::BoundingBox;
use cgmath::Point3;

#[derive(Default)]
pub struct Objects {
    pub list: Vec<Geometry>,
}

/// Parameter
#[derive(Clone, Copy, Debug)]
pub enum Parameter {
    /// Fixed value across all instances.
    Constant(f32),
    /// Reference into a parameter array.
    Symbolic(usize),
}

impl Parameter {
    pub fn value(&self, symbolic_values: &[f32]) -> Option<f32> {
        match self {
            Self::Constant(value) => Some(*value),
            Self::Symbolic(index) => symbolic_values.get(*index).copied(),
        }
    }
}

#[derive(Debug)]
pub enum Geometry {
    UnitSphere,
    UnitCube,
    Union {
        children: Vec<Box<Geometry>>,
    },
    Intersection {
        children: Vec<Box<Geometry>>,
    },
    Scale {
        factor: Parameter,
        f: Box<Geometry>,
    },
    /*Rotate {
        rotation: [Parameter; 4], // quaternion
        f: Box<Geometry>,
    },*/
    Translate {
        translation: [Parameter; 3],
        f: Box<Geometry>,
    },
    Round {
        radius: Parameter,
        f: Box<Geometry>,
    },
}

impl Geometry {
    /// Returns the estimated bounding box for an instance of this geometry, or
    /// `None` if a symbolic parameter was out of bounds of the provided array.
    pub fn bounding_box(&self, symbolic_values: &[f32]) -> Option<BoundingBox> {
        match self {
            Self::UnitSphere | Self::UnitCube => Some(BoundingBox {
                min: Point3::new(-1.0, -1.0, -1.0),
                max: Point3::new(1.0, 1.0, 1.0),
            }),
            // TODO: handle errors in a nicer way here??
            Self::Union { children } => Some(BoundingBox::union(
                children
                    .into_iter()
                    .map(|c| c.bounding_box(symbolic_values))
                    .collect::<Option<Vec<_>>>()?,
            )),
            Self::Intersection { children } => Some(BoundingBox::intersection(
                children
                    .into_iter()
                    .map(|c| c.bounding_box(symbolic_values))
                    .collect::<Option<Vec<_>>>()?,
            )),
            Self::Scale { factor, f } => {
                let BoundingBox { mut min, mut max } = f.bounding_box(symbolic_values)?;

                min *= factor.value(symbolic_values)?;
                max *= factor.value(symbolic_values)?;

                Some(BoundingBox { min, max })
            }
            Self::Translate { translation, f } => {
                let BoundingBox { mut min, mut max } = f.bounding_box(symbolic_values)?;

                min.x += translation[0].value(symbolic_values)?;
                min.y += translation[1].value(symbolic_values)?;
                min.z += translation[2].value(symbolic_values)?;
                max.x += translation[0].value(symbolic_values)?;
                max.y += translation[1].value(symbolic_values)?;
                max.z += translation[2].value(symbolic_values)?;

                Some(BoundingBox { min, max })
            }
            Self::Round { f, radius } => {
                let BoundingBox { mut min, mut max } = f.bounding_box(symbolic_values)?;

                let radius = radius.value(symbolic_values)?;

                min.x -= radius;
                min.y -= radius;
                min.z -= radius;
                max.x += radius;
                max.y += radius;
                max.z += radius;

                Some(BoundingBox { min, max })
            }
        }
    }
}
