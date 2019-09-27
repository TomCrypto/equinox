#[allow(unused_imports)]
use log::{debug, info, warn};

use cgmath::prelude::*;
use cgmath::Point3;

#[macro_export]
macro_rules! export {
    [$( $module:ident ),*] => {
        $(
            mod $module;
            pub use self::$module::*;
        )*
    };
}

export![
    camera,
    raster,
    object,
    instance,
    material,
    environment,
    display
];

/// Tracks mutable access to a value with a dirty flag.
///
/// The dirty flag is asserted whenever this type's `DerefMut` impl is
/// invoked and can be reset to `false` via the `Dirty::clean` method.
///
/// Wrapped values are initially considered dirty.
#[derive(Debug)]
pub struct Dirty<T> {
    is_dirty: bool,
    inner: T,
}

impl<T> Dirty<T> {
    /// Creates a new dirty value.
    pub fn new(inner: T) -> Self {
        Self {
            is_dirty: true,
            inner,
        }
    }

    /// Forcibly dirties the value.
    pub fn dirty(this: &mut Self) {
        this.is_dirty = true;
    }

    /// Marks the value as clean, invoking `callback` if it was dirty.
    pub fn clean(this: &mut Self, callback: impl FnOnce(&T)) -> bool {
        if !this.is_dirty {
            return false;
        }

        this.is_dirty = false;
        callback(&this.inner);

        true
    }
}

impl<T: Default> Default for Dirty<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> std::ops::Deref for Dirty<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for Dirty<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.is_dirty = true;

        &mut self.inner
    }
}

///
/// # Dirty Flags
///
/// For pragmatic reasons, the scene structure maintains dirty flags relative to
/// a particular device instance's internal state. As a consequence care must be
/// taken when rendering a scene on multiple devices simultaneously.
#[derive(Default)]
pub struct Scene {
    pub camera: Dirty<Camera>,
    pub raster: Dirty<Raster>,
    pub instances: Dirty<Instances>,
    pub objects: Dirty<Objects>,
    pub materials: Dirty<Materials>,
    pub environment: Dirty<Environment>,
    pub display: Dirty<Display>,
}

impl Scene {
    /// Creates a new empty scene with a default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks all of this scene as dirty, forcing a complete device update.
    ///
    /// This is normally only used internally by devices to respond to events
    /// such as device loss. However because the dirty flags stored by scenes
    /// are associated with a device's current state, you should call this if
    /// a scene is "moved" from one device to another (not recommended).
    pub fn dirty_all_fields(&mut self) {
        Dirty::dirty(&mut self.camera);
        Dirty::dirty(&mut self.raster);
        Dirty::dirty(&mut self.instances);
        Dirty::dirty(&mut self.objects);
        Dirty::dirty(&mut self.materials);
        Dirty::dirty(&mut self.environment);
        Dirty::dirty(&mut self.display);
    }
}

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
