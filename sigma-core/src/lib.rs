use cgmath::prelude::*;
use cgmath::{Decomposed, Point3, Quaternion, Vector3};
use itertools::{iproduct, izip};
use log::info;
use smart_default::SmartDefault;
use std::num::NonZeroU32;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

pub trait DeviceBuffer {
    fn map_update(&mut self, size: usize, f: impl FnOnce(&mut [u8]));
}

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

#[derive(Clone, Copy, SmartDefault)]
pub enum Aperture {
    #[default]
    Point,
    Circle {
        radius: f32,
    },
    Ngon {
        radius: f32,
        sides: u32,
        rotation: f32,
    },
}

impl Aperture {
    pub fn radius(&self) -> f32 {
        match self {
            Self::Point => 0.0,
            Self::Circle { radius } => *radius,
            Self::Ngon { radius, .. } => *radius,
        }
    }

    pub(crate) fn settings(&self) -> [f32; 4] {
        match self {
            Self::Point => [-1.0; 4],
            Self::Circle { .. } => [0.0, 0.0, 0.0, 0.0],
            Self::Ngon {
                sides, rotation, ..
            } => [1.0, *sides as f32, *rotation as f32, 1.0 / (*sides as f32)],
        }
    }
}

#[derive(SmartDefault)]
pub struct Camera {
    #[default(Point3::new(0.0, 0.0, 0.0))]
    pub position: Point3<f32>,

    #[default(Vector3::new(0.0, 0.0, 1.0))]
    pub direction: Vector3<f32>,

    #[default(Vector3::new(0.0, 1.0, 0.0))]
    pub up_vector: Vector3<f32>,

    #[default(Aperture::Point)]
    pub aperture: Aperture,

    #[default(1.0)]
    pub focal_distance: f32,

    #[default(0.06)]
    pub focal_length: f32,

    #[default(0.024)]
    pub film_height: f32,
}

impl Camera {
    pub fn update(&self, buffer: &mut impl DeviceBuffer) {
        #[repr(C)]
        #[derive(Default, AsBytes, FromBytes)]
        struct CameraData {
            origin_plane: [[f32; 4]; 4],
            target_plane: [[f32; 4]; 4],
            aperture_settings: [f32; 4],
        }

        buffer.map_update(std::mem::size_of::<CameraData>(), |memory| {
            let mut camera: LayoutVerified<_, CameraData> =
                LayoutVerified::new_zeroed(memory).unwrap();

            let fov_tan = self.film_height / (2.0 * self.focal_length);

            let mut xfm: cgmath::Matrix4<f32> = Transform::look_at(
                self.position,
                self.position + self.direction,
                self.up_vector,
            );

            xfm = xfm.inverse_transform().unwrap();

            for (&x, &y) in iproduct!(&[-1i32, 1i32], &[-1i32, 1i32]) {
                let plane_index = (y + 1 + (x + 1) / 2) as usize;

                let origin = xfm.transform_point(Point3::new(
                    (x as f32) * self.aperture.radius(),
                    (y as f32) * self.aperture.radius(),
                    0.0,
                ));

                let target = xfm.transform_point(Point3::new(
                    (x as f32) * fov_tan * self.focal_distance,
                    (y as f32) * fov_tan * self.focal_distance,
                    self.focal_distance,
                ));

                camera.origin_plane[plane_index] = [origin.x, origin.y, origin.z, 1.0];
                camera.target_plane[plane_index] = [target.x, target.y, target.z, 1.0];
            }

            camera.aperture_settings = self.aperture.settings();
        });
    }
}

#[derive(Copy, Clone, SmartDefault)]
pub enum RasterFilter {
    #[default]
    BlackmanHarris,
    Dirac,
}

impl RasterFilter {
    pub fn importance_sample(self, t: f32) -> f32 {
        match self {
            Self::Dirac => 0.0, // trivial window function
            _ => self.evaluate_inverse_cdf_via_bisection(t),
        }
    }

    #[allow(clippy::float_cmp)]
    fn evaluate_inverse_cdf_via_bisection(self, t: f32) -> f32 {
        let mut lo = 0.0;
        let mut hi = 1.0;
        let mut last = t;

        loop {
            let mid = (lo + hi) / 2.0;

            let sample = self.evaluate_cdf(mid);

            if sample == last {
                return mid;
            }

            if sample < t {
                lo = mid;
            } else {
                hi = mid;
            }

            last = sample;
        }
    }

    fn evaluate_cdf(self, t: f32) -> f32 {
        match self {
            Self::Dirac => unreachable!(),
            Self::BlackmanHarris => {
                let s1 = 0.216_623_8 * (2.0 * std::f32::consts::PI * t).sin();
                let s2 = 0.031_338_5 * (4.0 * std::f32::consts::PI * t).sin();
                let s3 = 0.001_727_2 * (6.0 * std::f32::consts::PI * t).sin();
                t - s1 + s2 - s3 // integral of the normalized window function
            }
        }
    }
}

#[derive(SmartDefault)]
pub struct Raster {
    #[default(NonZeroU32::new(256).unwrap())]
    pub width: NonZeroU32,
    #[default(NonZeroU32::new(256).unwrap())]
    pub height: NonZeroU32,
    pub filter: RasterFilter,
}

#[repr(C)]
#[derive(AsBytes, FromBytes)]
struct RasterData {
    width: f32,
    height: f32,
    inv_width: f32,
    inv_height: f32,
}

impl Raster {
    pub fn update_raster(&self, buffer: &mut impl DeviceBuffer) {
        buffer.map_update(std::mem::size_of::<RasterData>(), |memory| {
            let mut data: LayoutVerified<_, RasterData> = LayoutVerified::new(memory).unwrap();

            data.width = self.width.get() as f32;
            data.height = self.height.get() as f32;
            data.inv_width = 1.0 / data.width;
            data.inv_height = 1.0 / data.height;
        });
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
    pub fn dirty_all(&mut self) {
        Dirty::dirty(&mut self.camera);
        Dirty::dirty(&mut self.raster);
        Dirty::dirty(&mut self.instances);
        Dirty::dirty(&mut self.objects);
    }
}

pub struct Instance {
    pub object: usize,
    pub scale: f32,
    pub rotation: Quaternion<f32>,
    pub translation: Vector3<f32>,
}

#[repr(C)]
#[derive(Clone, Copy, FromBytes, AsBytes)]
struct InstanceData {
    transform: [f32; 12], // world transform for this instance
    hierarchy_start: u32, // where does the BVH start in the BVH data?
    hierarchy_limit: u32, // where does the BVH end? (as an absolute pos)
    triangles_start: u32, // where does the triangle data start?
    materials_start: u32, // where does the material data start? NOT IMPLEMENTED YET
}

// 16384 floats total available

// currently ew use 16 floats, so 1024 instances max...
// though we could possibly have more space (e.g. 65536 floats) so 4096
// instances max

// other data: material ranges? (per-instance). could replace with
// triangles_limit (don't need, possibly never will)
// anything else?

#[derive(Clone, Copy, Default, Debug)]
struct IndexData {
    hierarchy_start: u32,
    hierarchy_limit: u32,
    triangles_start: u32,
}

// TODO: this should build some kind of top-level scene BVH in addition to a
// linear array of instance elements
#[derive(Default)]
pub struct Instances {
    pub list: Vec<Instance>,
}

impl Instances {
    pub fn update(&self, objects: &Objects, buffer: &mut impl DeviceBuffer) {
        // method to load the "instance list" into a (uniform) buffer
        // for now we'll just iterate stupidly, later on throw in a BVH

        let indices = Self::calculate_indices(&objects.list);

        // TODO: better sizing of instance data
        // let size = self.list.len() * std::mem::size_of::<InstanceData>();
        let size = 128 * std::mem::size_of::<InstanceData>();

        buffer.map_update(size, |memory| {
            let mut slice: LayoutVerified<_, [InstanceData]> =
                LayoutVerified::new_slice_zeroed(memory).unwrap();

            for (memory, instance) in izip!(&mut *slice, &self.list) {
                let instance_world = Decomposed {
                    scale: instance.scale,
                    rot: instance.rotation,
                    disp: instance.translation,
                };

                if let Some(world_instance) = instance_world.inverse_transform() {
                    Self::pack_xfm_row_major(world_instance.into(), &mut memory.transform);
                }

                let index_data = &indices[instance.object];

                memory.hierarchy_start = index_data.hierarchy_start;
                memory.hierarchy_limit = index_data.hierarchy_limit;
                memory.triangles_start = index_data.triangles_start;
                memory.materials_start = 0;
            }
        });
    }

    fn calculate_indices(objects: &[Object]) -> Vec<IndexData> {
        let mut indices = Vec::with_capacity(objects.len());
        let mut current = IndexData::default();

        for object in objects {
            current.hierarchy_limit += object.hierarchy.len() as u32 / 32;

            indices.push(current);

            current.hierarchy_start += object.hierarchy.len() as u32 / 32;
            current.triangles_start += object.triangles.len() as u32 / 64;
        }

        indices
    }

    fn pack_xfm_row_major(xfm: cgmath::Matrix4<f32>, output: &mut [f32; 12]) {
        for (i, j) in iproduct!(0..4, 0..3) {
            output[4 * j + i] = xfm[i][j];
        }
    }
}

pub struct Object {
    pub hierarchy: Vec<u8>,
    pub triangles: Vec<u8>,
}

#[derive(Default)]
pub struct Objects {
    pub list: Vec<Object>,
}

impl Objects {
    pub fn update_hierarchy(&self, buffer: &mut impl DeviceBuffer) {
        buffer.map_update(self.hierarchy_data_size(), |mut memory| {
            for object in &self.list {
                let (region, rest) = memory.split_at_mut(object.hierarchy.len());
                region.copy_from_slice(&object.hierarchy);
                memory = rest;
            }
        });
    }

    pub fn update_triangles(&self, buffer: &mut impl DeviceBuffer) {
        buffer.map_update(self.triangles_data_size(), |mut memory| {
            for object in &self.list {
                let (region, rest) = memory.split_at_mut(object.triangles.len());
                region.copy_from_slice(&object.triangles);
                memory = rest;
            }
        });
    }

    // TODO: might be good to check for overflow here and stuff

    fn hierarchy_data_size(&self) -> usize {
        self.list.iter().map(|obj| obj.hierarchy.len()).sum()
    }

    fn triangles_data_size(&self) -> usize {
        self.list.iter().map(|obj| obj.triangles.len()).sum()
    }
}
