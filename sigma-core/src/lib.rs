#[allow(unused_imports)]
use log::{debug, info, warn};

use cgmath::prelude::*;
use cgmath::{Decomposed, Point3, Quaternion, Vector3};
use itertools::{iproduct, izip};
use smart_default::SmartDefault;
use std::mem::size_of;
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

    fn settings(&self) -> [f32; 4] {
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

        buffer.map_update(size_of::<CameraData>(), |memory| {
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
            Self::Dirac => 0.0, // dirac has a trivial CDF
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
        buffer.map_update(size_of::<RasterData>(), |memory| {
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
    hierarchy_limit: u32, // where does the BVH end? (as an absolute pos) - TODO: GET RID OF THIS!
    triangles_start: u32, // where does the triangle data start?
    materials_start: u32, // where does the material data start? NOT IMPLEMENTED YET
    vertices_start: u32,
    padding: [u32; 3],
}

#[derive(Clone, Copy, Default, Debug)]
struct IndexData {
    hierarchy_start: u32,
    hierarchy_limit: u32,
    triangles_start: u32,
    materials_start: u32,
    vertices_start: u32,
}

#[repr(align(64), C)]
#[derive(AsBytes, FromBytes)]
struct SceneHierarchyNode {
    lhs_bmin: [f32; 3],
    lhs_next: u32,
    lhs_bmax: [f32; 3],
    lhs_inst: u32,
    rhs_bmin: [f32; 3],
    rhs_next: u32,
    rhs_bmax: [f32; 3],
    rhs_inst: u32,
}

#[derive(Default)]
pub struct Instances {
    pub list: Vec<Instance>,
}

/// Builds an instance BVH for the scene.
struct SceneHierarchyBuilder<'a> {
    nodes: &'a mut [SceneHierarchyNode],
}

impl<'a> SceneHierarchyBuilder<'a> {
    pub fn new(nodes: &'a mut [SceneHierarchyNode]) -> Self {
        Self { nodes }
    }

    pub fn node_count_for_leaves(leaves: usize) -> usize {
        leaves.max(2) - 1 // need at least a root node
    }

    pub fn build(&mut self, leaves: &mut [InstanceInfo]) {
        if leaves.len() < 2 {
            // special case (tree has an incomplete root)
            return self.build_incomplete(leaves.first());
        }

        let total = self.build_recursive(0, leaves);
        assert_eq!(total as usize, self.nodes.len())
    }

    fn build_incomplete(&mut self, leaf: Option<&InstanceInfo>) {
        let node = &mut self.nodes[0];

        if let Some(leaf) = leaf {
            node.lhs_bmin = leaf.bbox.min.into();
            node.lhs_bmax = leaf.bbox.max.into();
            node.lhs_next = 0xffff_ffff;
            node.lhs_inst = leaf.inst;
        } else {
            node.lhs_bmin = [0.0, 0.0, 0.0];
            node.lhs_bmax = [0.0, 0.0, 0.0];
            node.lhs_next = 0xffff_ffff;
            node.lhs_inst = 0;
        }

        node.rhs_bmin = [0.0, 0.0, 0.0];
        node.rhs_bmax = [0.0, 0.0, 0.0];
        node.rhs_next = 0xffff_ffff;
        node.rhs_inst = 0;
    }

    fn build_recursive(&mut self, mut offset: u32, leaves: &mut [InstanceInfo]) -> u32 {
        let bbox = BoundingBox::from_extents(leaves.iter().map(|i| i.bbox));

        // TODO: implement SAH heuristic here when possible (need more info from object
        // BVHs). for now, do a median split on the largest axis

        let split = leaves.len() / 2;

        let dx = bbox.max.x - bbox.min.x;
        let dy = bbox.max.y - bbox.min.y;
        let dz = bbox.max.z - bbox.min.z;

        let mut axis = 0;

        if dx > dy && dx > dz {
            axis = 0;
        }

        if dy > dx && dy > dz {
            axis = 1;
        }

        if dz > dy && dz > dx {
            axis = 2;
        }

        leaves.sort_by_key(|instance| {
            let centroid = instance.bbox.centroid();

            match axis {
                0 => ordered_float::NotNan::new(centroid.x).unwrap(),
                1 => ordered_float::NotNan::new(centroid.y).unwrap(),
                2 => ordered_float::NotNan::new(centroid.z).unwrap(),
                _ => unreachable!(),
            }
        });

        let (lhs, rhs) = leaves.split_at_mut(split);

        let lhs_bbox = BoundingBox::from_extents(lhs.iter().map(|i| i.bbox));
        let rhs_bbox = BoundingBox::from_extents(rhs.iter().map(|i| i.bbox));

        let curr = offset as usize;
        offset += 1; // go to next

        self.nodes[curr].lhs_bmin = lhs_bbox.min.into();
        self.nodes[curr].lhs_bmax = lhs_bbox.max.into();
        self.nodes[curr].rhs_bmin = rhs_bbox.min.into();
        self.nodes[curr].rhs_bmax = rhs_bbox.max.into();

        if lhs.len() > 1 {
            self.nodes[curr].lhs_next = offset;
            self.nodes[curr].lhs_inst = 0;

            offset = self.build_recursive(offset, lhs);
        } else {
            self.nodes[curr].lhs_next = 0xffff_ffff;
            self.nodes[curr].lhs_inst = lhs[0].inst;
        }

        if rhs.len() > 1 {
            self.nodes[curr].rhs_next = offset;
            self.nodes[curr].rhs_inst = 0;

            offset = self.build_recursive(offset, rhs);
        } else {
            self.nodes[curr].rhs_next = 0xffff_ffff;
            self.nodes[curr].rhs_inst = rhs[0].inst;
        }

        offset
    }
}

struct InstanceInfo {
    bbox: BoundingBox,
    scale: f32,
    surface_area: f32,
    inst: u32,
}

impl Instances {
    pub fn update_scene_hierarchy(&self, objects: &Objects, buffer: &mut impl DeviceBuffer) {
        let mut instances = Vec::with_capacity(self.list.len());

        for (index, instance) in self.list.iter().enumerate() {
            let object = &objects.list[instance.object];

            let instance_world = Decomposed {
                scale: instance.scale,
                rot: instance.rotation,
                disp: instance.translation,
            };

            let bbox = object.bbox.transform(instance_world);

            instances.push(InstanceInfo {
                bbox,
                scale: instance.scale, // used to scale surface area heuristic
                surface_area: 1.0,     // obtain from object BVH
                inst: index as u32,
            });
        }

        let node_count = SceneHierarchyBuilder::node_count_for_leaves(self.list.len());

        buffer.map_update(node_count * size_of::<SceneHierarchyNode>(), |memory| {
            let slice = LayoutVerified::new_slice(memory).unwrap().into_mut_slice();

            SceneHierarchyBuilder::new(slice).build(&mut instances);
        });
    }

    pub fn update(&self, objects: &Objects, buffer: &mut impl DeviceBuffer) {
        let indices = Self::calculate_indices(&objects.list);

        buffer.map_update(self.list.len() * size_of::<InstanceData>(), |memory| {
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
                memory.materials_start = index_data.materials_start;
                memory.vertices_start = index_data.vertices_start;
            }
        });
    }

    fn calculate_indices(objects: &[Object]) -> Vec<IndexData> {
        let mut indices = Vec::with_capacity(objects.len());
        let mut current = IndexData::default();

        for object in objects {
            // TODO: drop hierarchy_limit, it won't be needed eventually
            current.hierarchy_limit += object.hierarchy.len() as u32 / 32;

            indices.push(current);

            current.hierarchy_start += object.hierarchy.len() as u32 / 32;
            current.triangles_start += object.triangles.len() as u32 / 16;
            current.materials_start += object.materials/*.len()*/ as u32;
            current.vertices_start += object.positions.len() as u32 / 16;
        }

        indices
    }

    fn pack_xfm_row_major(xfm: cgmath::Matrix4<f32>, output: &mut [f32; 12]) {
        for (i, j) in iproduct!(0..4, 0..3) {
            output[4 * j + i] = xfm[i][j];
        }
    }
}

#[derive(Clone, Copy)]
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

        let mut min = Point3::new(std::f32::INFINITY, std::f32::INFINITY, std::f32::INFINITY);
        let mut max = min * -1.0; // construct an invalid AABB to avoid needing an Option here

        for vertex in &vertices {
            let vertex = xfm.transform_point(*vertex);

            min.x = min.x.min(vertex.x);
            max.x = max.x.max(vertex.x);
            min.y = min.y.min(vertex.y);
            max.y = max.y.max(vertex.y);
            min.z = min.z.min(vertex.z);
            max.z = max.z.max(vertex.z);
        }

        Self { min, max }
    }

    pub fn from_extents(boxes: impl IntoIterator<Item = Self>) -> Self {
        let mut min = Point3::new(std::f32::INFINITY, std::f32::INFINITY, std::f32::INFINITY);
        let mut max = min * -1.0; // construct an invalid AABB to avoid needing an Option here

        for bbox in boxes.into_iter() {
            min.x = min.x.min(bbox.min.x);
            max.x = max.x.max(bbox.max.x);
            min.y = min.y.min(bbox.min.y);
            max.y = max.y.max(bbox.max.y);
            min.z = min.z.min(bbox.min.z);
            max.z = max.z.max(bbox.max.z);
        }

        Self { min, max }
    }
}

// TODO: use actual types for these things later on (with methods to get to them
// from a raw byte array for loading convenience of course... possibly defining
// a custom "all-in-one" object format)
pub struct Object {
    pub hierarchy: Vec<u8>,
    pub triangles: Vec<u8>,

    pub positions: Vec<u8>,
    pub normal_tangent_uv: Vec<u8>,
    pub materials: usize, // TODO: later on, specify default materials...

    pub bbox: BoundingBox,
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

    pub fn update_positions(&self, buffer: &mut impl DeviceBuffer) {
        buffer.map_update(self.positions_data_size(), |mut memory| {
            for object in &self.list {
                let (region, rest) = memory.split_at_mut(object.positions.len());
                region.copy_from_slice(&object.positions);
                memory = rest;
            }
        });
    }

    pub fn update_normal_tangent_uv(&self, buffer: &mut impl DeviceBuffer) {
        buffer.map_update(self.normal_tangent_uv_data_size(), |mut memory| {
            for object in &self.list {
                let (region, rest) = memory.split_at_mut(object.normal_tangent_uv.len());
                region.copy_from_slice(&object.normal_tangent_uv);
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

    fn positions_data_size(&self) -> usize {
        self.list.iter().map(|obj| obj.positions.len()).sum()
    }

    fn normal_tangent_uv_data_size(&self) -> usize {
        self.list
            .iter()
            .map(|obj| obj.normal_tangent_uv.len())
            .sum()
    }
}
