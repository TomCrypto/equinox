use crate::BoundingBox;
use cgmath::prelude::*;
use cgmath::{Matrix3, Point3, Rad, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Parameter
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Parameter {
    /// Fixed value across all instances.
    Constant(f32),
    /// Reference into a parameter table.
    Symbolic(String),
}

impl Parameter {
    pub fn value(&self, symbolic_values: &BTreeMap<String, f32>) -> f32 {
        match self {
            Self::Constant(number) => *number,
            Self::Symbolic(symbol) => symbolic_values[symbol],
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Geometry {
    Sphere {
        radius: Parameter,
    },
    Ellipsoid {
        radius: [Parameter; 3],
    },
    Cuboid {
        dimensions: [Parameter; 3],
    },
    Cylinder {
        height: Parameter,
        radius: Parameter,
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
    Onion {
        thickness: Parameter,
        f: Box<Geometry>,
    },
    Scale {
        factor: Parameter,
        f: Box<Geometry>,
    },
    Rotate {
        axis: [Parameter; 3],
        angle: Parameter,
        f: Box<Geometry>,
    },
    Translate {
        translation: [Parameter; 3],
        f: Box<Geometry>,
    },
    Round {
        radius: Parameter,
        f: Box<Geometry>,
    },
    ForceNumericalNormals {
        f: Box<Geometry>,
    },
}

impl Geometry {
    /// Returns the approximate cost of evaluating the distance field, based on
    /// the arbitrary measure that the evaluation cost of the unit sphere is 1.
    pub fn evaluation_cost(&self) -> f32 {
        match self {
            Self::Sphere { .. } => 1.0,
            Self::Ellipsoid { .. } => 1.0,
            Self::Cuboid { .. } => 1.5,
            Self::Cylinder { .. } => 2.0,
            Self::InfiniteRepetition { f, .. } => 0.5 + f.evaluation_cost(),
            Self::Union { children } => children.iter().map(|x| 0.25 + x.evaluation_cost()).sum(),
            Self::Intersection { children } => {
                children.iter().map(|x| 0.5 + x.evaluation_cost()).sum()
            }
            Self::Subtraction { lhs, rhs } => lhs.evaluation_cost() + rhs.evaluation_cost() + 0.25,
            Self::Onion { f, .. } => f.evaluation_cost() + 0.25,
            Self::Scale { f, .. } => f.evaluation_cost() + 1.0,
            Self::Rotate { f, .. } => f.evaluation_cost() + 2.0,
            Self::Translate { f, .. } => f.evaluation_cost() + 0.25,
            Self::Round { f, .. } => f.evaluation_cost() + 0.25,
            Self::ForceNumericalNormals { f } => f.evaluation_cost(),
        }
    }

    /// Returns a bounding box for an instance of this geometry, or panics if
    /// some referenced parameter values are absent from the parameter table.
    pub fn bounding_box(&self, parameters: &BTreeMap<String, f32>) -> BoundingBox {
        match self {
            Self::Sphere { radius } => {
                let radius = radius.value(parameters);

                BoundingBox {
                    min: [-radius; 3].into(),
                    max: [radius; 3].into(),
                }
            }
            Self::Ellipsoid { radius } => {
                let mut radius_x = radius[0].value(parameters);
                let mut radius_y = radius[1].value(parameters);
                let mut radius_z = radius[2].value(parameters);

                // TODO: we need to do this to account for the fact that this SDF is a bound, is
                // there a better way to implement this or are we stuck with this approximation?

                let min_radius = radius_x.min(radius_y).min(radius_z);

                radius_x *= radius_x / min_radius;
                radius_y *= radius_y / min_radius;
                radius_z *= radius_z / min_radius;

                BoundingBox {
                    min: [-radius_x, -radius_y, -radius_z].into(),
                    max: [radius_x, radius_y, radius_z].into(),
                }
            }
            Self::Cuboid { dimensions } => {
                let dim_x = dimensions[0].value(parameters);
                let dim_y = dimensions[1].value(parameters);
                let dim_z = dimensions[2].value(parameters);

                BoundingBox {
                    min: [-dim_x, -dim_y, -dim_z].into(),
                    max: [dim_x, dim_y, dim_z].into(),
                }
            }
            Self::Cylinder { height, radius } => {
                let height = height.value(parameters);
                let radius = radius.value(parameters);

                BoundingBox {
                    min: [-radius, -height, -radius].into(),
                    max: [radius, height, radius].into(),
                }
            }
            // TODO: this is wrong (also we should bound repetition anyway)
            Self::InfiniteRepetition { .. } => BoundingBox {
                min: Point3::new(-100.0, -2.0, -100.0),
                max: Point3::new(100.0, 2.0, 100.0),
            },
            Self::Union { children } => {
                let mut bbox = BoundingBox::neg_infinity_bounds();

                for child in children {
                    bbox.extend(&child.bounding_box(parameters));
                }

                bbox
            }
            Self::Intersection { children } => {
                let mut bbox = BoundingBox::pos_infinity_bounds();

                for child in children {
                    bbox.intersect(&child.bounding_box(parameters));
                }

                bbox
            }
            Self::Subtraction { lhs, .. } => lhs.bounding_box(parameters),
            Self::Onion { thickness, f } => {
                let BoundingBox { mut min, mut max } = f.bounding_box(parameters);

                let thickness = thickness.value(parameters);

                min.x -= thickness;
                min.y -= thickness;
                min.z -= thickness;
                max.x += thickness;
                max.y += thickness;
                max.z += thickness;

                BoundingBox { min, max }
            }
            Self::Scale { factor, f } => {
                let BoundingBox { mut min, mut max } = f.bounding_box(parameters);

                min *= factor.value(parameters);
                max *= factor.value(parameters);

                BoundingBox { min, max }
            }
            Self::Rotate { axis, angle, f } => {
                let rotation_axis: Vector3<f32> = [
                    axis[0].value(parameters),
                    axis[1].value(parameters),
                    axis[2].value(parameters),
                ]
                .into();

                let rotation = Matrix3::from_axis_angle(
                    rotation_axis.normalize(),
                    Rad(angle.value(parameters)),
                );

                f.bounding_box(parameters).transform(rotation)
            }
            Self::Translate { translation, f } => {
                let BoundingBox { mut min, mut max } = f.bounding_box(parameters);

                min.x += translation[0].value(parameters);
                min.y += translation[1].value(parameters);
                min.z += translation[2].value(parameters);
                max.x += translation[0].value(parameters);
                max.y += translation[1].value(parameters);
                max.z += translation[2].value(parameters);

                BoundingBox { min, max }
            }
            Self::Round { f, radius } => {
                let BoundingBox { mut min, mut max } = f.bounding_box(parameters);

                let radius = radius.value(parameters);

                min.x -= radius;
                min.y -= radius;
                min.z -= radius;
                max.x += radius;
                max.y += radius;
                max.z += radius;

                BoundingBox { min, max }
            }
            Self::ForceNumericalNormals { f } => f.bounding_box(parameters),
        }
    }

    /// Returns a vector of all symbolic parameters found in this geometry in
    /// a deterministic order, representing the approximate evaluation order.
    pub fn symbolic_parameters(&self) -> Vec<&str> {
        let mut parameters = vec![];

        self.symbolic_parameters_recursive(&mut parameters);

        parameters
    }

    fn record_parameter<'a>(parameters: &mut Vec<&'a str>, parameter: &'a Parameter) {
        if let Parameter::Symbolic(symbol) = parameter {
            parameters.push(symbol);
        }
    }

    fn symbolic_parameters_recursive<'a>(&'a self, parameters: &mut Vec<&'a str>) {
        match self {
            Self::Sphere { radius } => {
                Self::record_parameter(parameters, radius);
            }
            Self::Ellipsoid { radius } => {
                Self::record_parameter(parameters, &radius[0]);
                Self::record_parameter(parameters, &radius[1]);
                Self::record_parameter(parameters, &radius[2]);
            }
            Self::Cuboid { dimensions } => {
                Self::record_parameter(parameters, &dimensions[0]);
                Self::record_parameter(parameters, &dimensions[1]);
                Self::record_parameter(parameters, &dimensions[2]);
            }
            Self::Cylinder { height, radius } => {
                Self::record_parameter(parameters, height);
                Self::record_parameter(parameters, radius);
            }
            Self::InfiniteRepetition { f, period } => {
                Self::record_parameter(parameters, &period[0]);
                Self::record_parameter(parameters, &period[1]);
                Self::record_parameter(parameters, &period[2]);

                f.symbolic_parameters_recursive(parameters);
            }
            Self::Union { children } | Self::Intersection { children } => {
                for child in children {
                    child.symbolic_parameters_recursive(parameters);
                }
            }
            Self::Subtraction { lhs, rhs } => {
                lhs.symbolic_parameters_recursive(parameters);
                rhs.symbolic_parameters_recursive(parameters);
            }
            Self::Onion { thickness, f } => {
                Self::record_parameter(parameters, thickness);

                f.symbolic_parameters_recursive(parameters);
            }
            Self::Scale { factor, f } => {
                Self::record_parameter(parameters, factor);

                f.symbolic_parameters_recursive(parameters);
            }
            Self::Rotate { axis, angle, f } => {
                Self::record_parameter(parameters, &axis[0]);
                Self::record_parameter(parameters, &axis[1]);
                Self::record_parameter(parameters, &axis[2]);
                Self::record_parameter(parameters, angle);

                f.symbolic_parameters_recursive(parameters);
            }
            Self::Translate { translation, f } => {
                Self::record_parameter(parameters, &translation[0]);
                Self::record_parameter(parameters, &translation[1]);
                Self::record_parameter(parameters, &translation[2]);

                f.symbolic_parameters_recursive(parameters);
            }
            Self::Round { f, radius } => {
                Self::record_parameter(parameters, radius);

                f.symbolic_parameters_recursive(parameters);
            }
            Self::ForceNumericalNormals { f } => {
                f.symbolic_parameters_recursive(parameters);
            }
        }
    }
}
