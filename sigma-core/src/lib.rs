use cgmath::prelude::*;
use cgmath::{vec3, Decomposed, Matrix3, Point3, Quaternion, Vector3};
use itertools::{iproduct, izip};
use log::info;
use std::num::NonZeroU32;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

pub trait DeviceBuffer {
    fn map_update(&mut self, size: usize, f: impl FnOnce(&mut [u8]));
}

// for now no origin
#[derive(Default)]
pub struct Camera {
    pub angle_x: f32,
    pub angle_y: f32,
    pub fov: f32,
    pub distance: f32,
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

impl Camera {
    pub fn zoom(&mut self, factor: f32) {
        self.distance *= factor;
    }

    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        self.angle_x += delta_x;
        self.angle_y += delta_y;

        if self.angle_y > std::f32::consts::PI - 0.01 {
            self.angle_y = std::f32::consts::PI - 0.01;
        }

        if self.angle_y < 0.01 {
            self.angle_y = 0.01;
        }
    }

    pub fn update(&self, buffer: &mut impl DeviceBuffer) {
        buffer.map_update(std::mem::size_of::<CameraData>(), |memory| {
            let x = self.angle_y.sin() * self.angle_x.cos();
            let z = self.angle_y.sin() * self.angle_x.sin();
            let y = self.angle_y.cos();

            let xfm = <Matrix3<f32> as Transform<Point3<f32>>>::look_at(
                Point3::new(x, y, z),
                Point3::new(0.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
            )
            .invert()
            .unwrap();

            // generate four camera points
            let fz = 1.0 / (self.fov * 0.5).tan();

            let mut layout: LayoutVerified<_, CameraData> =
                LayoutVerified::new_zeroed(memory).unwrap();

            layout.pos = [self.distance * x, self.distance * y, self.distance * z, 0.0];
            layout.fp0 = pack_vec3(Transform::<Point3<f32>>::transform_vector(
                &xfm,
                vec3(-1.0, 1.0, fz),
            ));
            layout.fp1 = pack_vec3(Transform::<Point3<f32>>::transform_vector(
                &xfm,
                vec3(1.0, 1.0, fz),
            ));
            layout.fp2 = pack_vec3(Transform::<Point3<f32>>::transform_vector(
                &xfm,
                vec3(-1.0, -1.0, fz),
            ));
            layout.fp3 = pack_vec3(Transform::<Point3<f32>>::transform_vector(
                &xfm,
                vec3(1.0, -1.0, fz),
            ));
        });
    }
}

trait Pack {
    type Data;

    fn pack(&self) -> Self::Data;
}

fn pack_vec3(v: Vector3<f32>) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}

impl Pack for Vector3<f32> {
    type Data = [f32; 4];

    fn pack(&self) -> Self::Data {
        [self.x, self.y, self.z, 0.0]
    }
}

#[repr(C)]
#[derive(Default, AsBytes, FromBytes)]
struct CameraData {
    fp0: [f32; 4],
    fp1: [f32; 4],
    fp2: [f32; 4],
    fp3: [f32; 4],
    pos: [f32; 4],
}

pub struct Frame {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub seed: u64,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            width: NonZeroU32::new(256).unwrap(),
            height: NonZeroU32::new(256).unwrap(),
            seed: 0,
        }
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
    pub frame: Dirty<Frame>,
    pub instances: Dirty<Instances>,
    pub objects: Dirty<Objects>,
}

impl Scene {
    /// Creates a new empty scene with a default configuration.
    pub fn new() -> Self {
        // TODO: this should just be a call to Default in theory
        // remove all this workaround logic when possible

        let mut s = Self {
            camera: Dirty::new(Camera::default()),
            ..Default::default()
        };

        s.camera.distance = 1050.0;
        s.camera.fov = std::f32::consts::PI / 3.0;
        s.camera.angle_x = 0.491;
        s.camera.angle_y = 0.223;

        s
    }

    /// Marks all of this scene as dirty, forcing a complete device update.
    ///
    /// This is normally only used internally by devices to respond to events
    /// such as device loss. However because the dirty flags stored by scenes
    /// are associated with a device's current state, you should call this if
    /// a scene is "moved" from one device to another (not recommended).
    pub fn dirty_all(&mut self) {
        Dirty::dirty(&mut self.camera);
        Dirty::dirty(&mut self.frame);
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
