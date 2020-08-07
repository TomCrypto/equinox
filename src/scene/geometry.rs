use crate::BoundingBox;
use cgmath::prelude::*;
use cgmath::{Matrix3, Rad, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// GeometryParameter
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum GeometryParameter {
    /// Fixed value across all instances.
    Constant(f32),
    /// Reference into a parameter table.
    Symbolic(String),
}

impl GeometryParameter {
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
        radius: GeometryParameter,
    },
    Ellipsoid {
        radius: [GeometryParameter; 3],
    },
    Cuboid {
        dimensions: [GeometryParameter; 3],
    },
    Cylinder {
        height: GeometryParameter,
        radius: GeometryParameter,
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
        thickness: GeometryParameter,
        child: Box<Geometry>,
    },
    Scale {
        factor: GeometryParameter,
        child: Box<Geometry>,
    },
    Rotate {
        axis: [GeometryParameter; 3],
        angle: GeometryParameter,
        child: Box<Geometry>,
    },
    Translate {
        translation: [GeometryParameter; 3],
        child: Box<Geometry>,
    },
    Round {
        radius: GeometryParameter,
        child: Box<Geometry>,
    },
    ForceNumericalNormals {
        child: Box<Geometry>,
    },
    CustomModifier {
        code: String,
        expansion: [f32; 3],
        child: Box<Geometry>,
    },
    Twist {
        amount: GeometryParameter,
        step: GeometryParameter,
        child: Box<Geometry>,
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
            Self::Union { children } => children.iter().map(|x| 0.25 + x.evaluation_cost()).sum(),
            Self::Intersection { children } => {
                children.iter().map(|x| 0.5 + x.evaluation_cost()).sum()
            }
            Self::Subtraction { lhs, rhs } => lhs.evaluation_cost() + rhs.evaluation_cost() + 0.25,
            Self::Onion { child, .. } => child.evaluation_cost() + 0.25,
            Self::Scale { child, .. } => child.evaluation_cost() + 1.0,
            Self::Rotate { child, .. } => child.evaluation_cost() + 2.0,
            Self::Translate { child, .. } => child.evaluation_cost() + 0.25,
            Self::Round { child, .. } => child.evaluation_cost() + 0.25,
            Self::ForceNumericalNormals { child } => child.evaluation_cost(),
            Self::CustomModifier { child, .. } => child.evaluation_cost() + 3.0,
            Self::Twist { child, .. } => child.evaluation_cost() + 1.0,
        }
    }

    /// Returns a bounding box for an instance of this geometry, or panics if
    /// some referenced parameter values are absent from the parameter table.
    pub fn bounding_box(&self, parameters: &BTreeMap<String, f32>) -> BoundingBox {
        self.bounds(parameters).bbox
    }

    fn bounds(&self, parameters: &BTreeMap<String, f32>) -> GeometryBounds {
        match self {
            Self::Sphere { radius } => {
                let radius = radius.value(parameters);

                GeometryBounds {
                    bbox: BoundingBox {
                        min: [-radius; 3].into(),
                        max: [radius; 3].into(),
                    },
                    scale_factor: 1.0,
                }
            }
            Self::Ellipsoid { radius } => {
                let radius_x = radius[0].value(parameters);
                let radius_y = radius[1].value(parameters);
                let radius_z = radius[2].value(parameters);

                let max_radius = radius_x.max(radius_y).max(radius_z);
                let min_radius = radius_x.min(radius_y).min(radius_z);

                GeometryBounds {
                    bbox: BoundingBox {
                        min: [-radius_x, -radius_y, -radius_z].into(),
                        max: [radius_x, radius_y, radius_z].into(),
                    },
                    scale_factor: max_radius / min_radius,
                }
            }
            Self::Cuboid { dimensions } => {
                let dim_x = dimensions[0].value(parameters);
                let dim_y = dimensions[1].value(parameters);
                let dim_z = dimensions[2].value(parameters);

                GeometryBounds {
                    bbox: BoundingBox {
                        min: [-dim_x, -dim_y, -dim_z].into(),
                        max: [dim_x, dim_y, dim_z].into(),
                    },
                    scale_factor: 1.0,
                }
            }
            Self::Cylinder { height, radius } => {
                let height = height.value(parameters);
                let radius = radius.value(parameters);

                GeometryBounds {
                    bbox: BoundingBox {
                        min: [-radius, -height, -radius].into(),
                        max: [radius, height, radius].into(),
                    },
                    scale_factor: 1.0,
                }
            }
            Self::Union { children } => {
                let mut bbox = BoundingBox::neg_infinity_bounds();
                let mut scale_factor: f32 = 0.0;

                for child in children {
                    let bounds = child.bounds(parameters);

                    scale_factor = scale_factor.max(bounds.scale_factor);
                    bbox.extend(&bounds.bbox);
                }

                GeometryBounds { bbox, scale_factor }
            }
            Self::Intersection { children } => {
                let mut bbox = BoundingBox::pos_infinity_bounds();
                let mut scale_factor: f32 = 0.0;

                for child in children {
                    let bounds = child.bounds(parameters);

                    scale_factor = scale_factor.max(bounds.scale_factor);
                    bbox.intersect(&bounds.bbox);
                }

                GeometryBounds { bbox, scale_factor }
            }
            Self::Subtraction { lhs, .. } => lhs.bounds(parameters),
            Self::Onion { thickness, child } => {
                let mut bounds = child.bounds(parameters);

                let thickness = thickness.value(parameters) * bounds.scale_factor;

                bounds.bbox.min.x -= thickness;
                bounds.bbox.min.y -= thickness;
                bounds.bbox.min.z -= thickness;
                bounds.bbox.max.x += thickness;
                bounds.bbox.max.y += thickness;
                bounds.bbox.max.z += thickness;

                bounds
            }
            Self::Scale { factor, child } => {
                let mut bounds = child.bounds(parameters);

                bounds.bbox.min *= factor.value(parameters);
                bounds.bbox.max *= factor.value(parameters);

                bounds
            }
            Self::Rotate { axis, angle, child } => {
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

                let mut bounds = child.bounds(parameters);

                bounds.bbox = bounds.bbox.transform(rotation);

                bounds
            }
            Self::Translate { translation, child } => {
                let mut bounds = child.bounds(parameters);

                bounds.bbox.min.x += translation[0].value(parameters);
                bounds.bbox.min.y += translation[1].value(parameters);
                bounds.bbox.min.z += translation[2].value(parameters);
                bounds.bbox.max.x += translation[0].value(parameters);
                bounds.bbox.max.y += translation[1].value(parameters);
                bounds.bbox.max.z += translation[2].value(parameters);

                bounds
            }
            Self::Round { child, radius } => {
                let mut bounds = child.bounds(parameters);

                let radius = radius.value(parameters) * bounds.scale_factor;

                bounds.bbox.min.x -= radius;
                bounds.bbox.min.y -= radius;
                bounds.bbox.min.z -= radius;
                bounds.bbox.max.x += radius;
                bounds.bbox.max.y += radius;
                bounds.bbox.max.z += radius;

                bounds
            }
            Self::ForceNumericalNormals { child } => child.bounds(parameters),
            Self::CustomModifier {
                child, expansion, ..
            } => {
                let mut bounds = child.bounds(parameters);

                bounds.bbox.min.x -= expansion[0];
                bounds.bbox.min.y -= expansion[1];
                bounds.bbox.min.z -= expansion[2];

                bounds.bbox.max.x += expansion[0];
                bounds.bbox.max.y += expansion[1];
                bounds.bbox.max.z += expansion[2];

                bounds
            }
            Self::Twist { child, .. } => {
                let mut bounds = child.bounds(parameters);

                let dx = bounds.bbox.max.x - bounds.bbox.min.x;
                let dy = bounds.bbox.max.y - bounds.bbox.min.y;
                let dz = bounds.bbox.max.z - bounds.bbox.min.z;

                bounds.bbox.min.x -= dx * 0.5;
                bounds.bbox.min.y -= dy * 0.5;
                bounds.bbox.min.z -= dz * 0.5;

                bounds.bbox.max.x += dx * 0.5;
                bounds.bbox.max.y += dy * 0.5;
                bounds.bbox.max.z += dz * 0.5;

                bounds
            }
        }
    }

    /// Returns a vector of all symbolic parameters found in this geometry in
    /// a deterministic order, representing the approximate evaluation order.
    pub fn symbolic_parameters(&self) -> Vec<&str> {
        let mut parameters = vec![];

        self.symbolic_parameters_recursive(&mut parameters);

        parameters
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
            Self::Union { children } | Self::Intersection { children } => {
                for child in children {
                    child.symbolic_parameters_recursive(parameters);
                }
            }
            Self::Subtraction { lhs, rhs } => {
                lhs.symbolic_parameters_recursive(parameters);
                rhs.symbolic_parameters_recursive(parameters);
            }
            Self::Onion { thickness, child } => {
                Self::record_parameter(parameters, thickness);

                child.symbolic_parameters_recursive(parameters);
            }
            Self::Scale { factor, child } => {
                Self::record_parameter(parameters, factor);

                child.symbolic_parameters_recursive(parameters);
            }
            Self::Rotate { axis, angle, child } => {
                Self::record_parameter(parameters, &axis[0]);
                Self::record_parameter(parameters, &axis[1]);
                Self::record_parameter(parameters, &axis[2]);
                Self::record_parameter(parameters, angle);

                child.symbolic_parameters_recursive(parameters);
            }
            Self::Translate { translation, child } => {
                Self::record_parameter(parameters, &translation[0]);
                Self::record_parameter(parameters, &translation[1]);
                Self::record_parameter(parameters, &translation[2]);

                child.symbolic_parameters_recursive(parameters);
            }
            Self::Round { child, radius } => {
                Self::record_parameter(parameters, radius);

                child.symbolic_parameters_recursive(parameters);
            }
            Self::ForceNumericalNormals { child } => {
                child.symbolic_parameters_recursive(parameters);
            }
            Self::CustomModifier { child, .. } => {
                child.symbolic_parameters_recursive(parameters);
            }
            Self::Twist {
                child,
                amount,
                step,
            } => {
                Self::record_parameter(parameters, amount);
                Self::record_parameter(parameters, step);

                child.symbolic_parameters_recursive(parameters);
            }
        }
    }

    fn record_parameter<'a>(parameters: &mut Vec<&'a str>, parameter: &'a GeometryParameter) {
        if let GeometryParameter::Symbolic(symbol) = parameter {
            parameters.push(symbol);
        }
    }
}

#[derive(Debug)]
struct GeometryBounds {
    bbox: BoundingBox,
    scale_factor: f32,
}
