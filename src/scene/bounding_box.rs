use cgmath::prelude::*;
use cgmath::Point3;

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub min: Point3<f32>,
    pub max: Point3<f32>,
}

impl BoundingBox {
    pub fn centroid(&self) -> Point3<f32> {
        self.min + (self.max - self.min) / 2.0
    }

    pub fn transform(&self, xfm: impl Transform<Point3<f32>>) -> Self {
        let vertices = [
            Point3::new(self.min.x, self.min.y, self.min.z),
            Point3::new(self.min.x, self.min.y, self.max.z),
            Point3::new(self.min.x, self.max.y, self.min.z),
            Point3::new(self.min.x, self.max.y, self.max.z),
            Point3::new(self.max.x, self.min.y, self.min.z),
            Point3::new(self.max.x, self.min.y, self.max.z),
            Point3::new(self.max.x, self.max.y, self.min.z),
            Point3::new(self.max.x, self.max.y, self.max.z),
        ];

        Self::from_extents(vertices.iter().map(|&vertex| {
            // find the new bounding box for all vertices
            Self::from_point(xfm.transform_point(vertex))
        }))
    }

    pub fn from_point(point: Point3<f32>) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    pub fn union(boxes: impl IntoIterator<Item = Self>) -> Self {
        Self::from_extents(boxes)
    }

    pub fn intersection(boxes: impl IntoIterator<Item = Self>) -> Self {
        let max = Point3::new(std::f32::INFINITY, std::f32::INFINITY, std::f32::INFINITY);
        let min = max * -1.0; // this ensures that any min/max operation updates the bbox

        let mut extents = Self { max, min };

        for bbox in boxes.into_iter() {
            extents.min.x = extents.min.x.max(bbox.min.x);
            extents.min.y = extents.min.y.max(bbox.min.y);
            extents.min.z = extents.min.z.max(bbox.min.z);
            extents.max.x = extents.max.x.min(bbox.max.x);
            extents.max.y = extents.max.y.min(bbox.max.y);
            extents.max.z = extents.max.z.min(bbox.max.z);
        }

        extents
    }

    pub fn from_extents(boxes: impl IntoIterator<Item = Self>) -> Self {
        let mut extents = Self::make_invalid_bbox();

        for bbox in boxes.into_iter() {
            extents.min.x = extents.min.x.min(bbox.min.x);
            extents.max.x = extents.max.x.max(bbox.max.x);
            extents.min.y = extents.min.y.min(bbox.min.y);
            extents.max.y = extents.max.y.max(bbox.max.y);
            extents.min.z = extents.min.z.min(bbox.min.z);
            extents.max.z = extents.max.z.max(bbox.max.z);
        }

        extents
    }

    fn make_invalid_bbox() -> Self {
        let min = Point3::new(std::f32::INFINITY, std::f32::INFINITY, std::f32::INFINITY);
        let max = min * -1.0; // this ensures that any min/max operation updates the bbox

        Self { min, max }
    }
}