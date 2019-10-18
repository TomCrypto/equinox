use crate::BoundingBox;
use cgmath::Point3;
use serde::{Deserialize, Serialize};

/// Parameter
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Parameter {
    /// Fixed value across all instances.
    Constant { value: f32 },
    /// Reference into a parameter array.
    Symbolic { index: usize },
}

impl Parameter {
    pub fn value(&self, symbolic_values: &[f32]) -> Option<f32> {
        match self {
            Self::Constant { value } => Some(*value),
            Self::Symbolic { index } => symbolic_values.get(*index).copied(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Geometry {
    Sphere {
        radius: Parameter,
    },
    Cuboid {
        dimensions: [Parameter; 3],
    },
    Plane {
        width: Parameter,
        length: Parameter,
    },
    InfiniteRepetition {
        f: Box<Geometry>,
        period: [Parameter; 3],
    },
    Union {
        children: Vec<Geometry>,
    },
    Intersection {
        children: Vec<Geometry>,
    },
    Subtraction {
        lhs: Box<Geometry>,
        rhs: Box<Geometry>,
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
    /// Returns the approximate cost of evaluating the distance field, based on
    /// the arbitrary measure that the evaluation cost of the unit sphere is 1.
    pub fn evaluation_cost(&self) -> f32 {
        match self {
            Self::Sphere { .. } | Self::Plane { .. } => 1.0,
            Self::Cuboid { .. } => 1.5,
            Self::InfiniteRepetition { f, .. } => 0.5 + f.evaluation_cost(),
            Self::Union { children } => children.iter().map(|x| 0.25 + x.evaluation_cost()).sum(),
            Self::Intersection { children } => {
                children.iter().map(|x| 0.5 + x.evaluation_cost()).sum()
            }
            Self::Subtraction { lhs, rhs } => lhs.evaluation_cost() + rhs.evaluation_cost() + 0.25,
            Self::Scale { f, .. } => f.evaluation_cost() + 1.0,
            Self::Translate { f, .. } => f.evaluation_cost() + 0.25,
            Self::Round { f, .. } => f.evaluation_cost() + 0.25,
        }
    }

    /// Returns the estimated bounding box for an instance of this geometry, or
    /// `None` if a symbolic parameter was out of bounds of the provided array.
    pub fn bounding_box(&self, symbolic_values: &[f32]) -> Option<BoundingBox> {
        match self {
            Self::Sphere { radius } => {
                let radius = radius.value(symbolic_values)?;

                Some(BoundingBox {
                    min: [-radius; 3].into(),
                    max: [radius; 3].into(),
                })
            }
            Self::Cuboid { dimensions } => {
                let dim_x = dimensions[0].value(symbolic_values)?;
                let dim_y = dimensions[1].value(symbolic_values)?;
                let dim_z = dimensions[2].value(symbolic_values)?;

                Some(BoundingBox {
                    min: [-dim_x, -dim_y, -dim_z].into(),
                    max: [dim_x, dim_y, dim_z].into(),
                })
            }
            Self::Plane { width, length } => {
                let width = width.value(symbolic_values)?;
                let length = length.value(symbolic_values)?;

                Some(BoundingBox {
                    min: Point3::new(-width, 0.0, -length),
                    max: Point3::new(width, 0.0, length),
                })
            }
            // TODO: this is wrong (also we should bound repetition anyway)
            Self::InfiniteRepetition { .. } => Some(BoundingBox {
                min: Point3::new(-100.0, -2.0, -100.0),
                max: Point3::new(100.0, 2.0, 100.0),
            }),
            Self::Union { children } => {
                let mut bbox = BoundingBox::neg_infinity_bounds();

                for child in children {
                    bbox.extend(&child.bounding_box(symbolic_values)?);
                }

                Some(bbox)
            }
            Self::Intersection { children } => {
                let mut bbox = BoundingBox::pos_infinity_bounds();

                for child in children {
                    bbox.intersect(&child.bounding_box(symbolic_values)?);
                }

                Some(bbox)
            }
            Self::Subtraction { lhs, .. } => lhs.bounding_box(symbolic_values),
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
