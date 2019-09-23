#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::device::ToDevice;
use crate::model::{Geometry, Instances};
use crate::BoundingBox;
use cgmath::prelude::*;
use cgmath::Decomposed;
use cgmath::Point3;
use itertools::{iproduct, izip};
use zerocopy::{AsBytes, FromBytes};

#[derive(Clone, Copy, Default, Debug)]
pub struct IndexData {
    geometry_offset: u32, // offset into the parameter array
}

#[repr(align(32), C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct SceneHierarchyNode {
    bmin: [f32; 3],
    packed1: u32, // "skip" pointer + "parameter offset" << 16
    bmax: [f32; 3],
    packed2: u32, // geometry ID + material ID << 16  == 0 if it's not a leaf
}

#[derive(Debug)]
struct InstanceInfo {
    bbox: BoundingBox,
    surface_area: f32, // TODO: can we ever actually compute this? might be doable actually
    geometry: u16,
    material: u16,
    geo_start: u16,
    mat_start: u16,
}

#[repr(transparent)]
#[derive(AsBytes, FromBytes, Copy, Clone)]
pub struct MaterialIndex([u32; 4]);

/// Builds an instance BVH for the scene.
struct SceneHierarchyBuilder<'a> {
    nodes: &'a mut [SceneHierarchyNode],
}

impl<'a> SceneHierarchyBuilder<'a> {
    pub fn new(nodes: &'a mut [SceneHierarchyNode]) -> Self {
        Self { nodes }
    }

    pub fn node_count_for_leaves(leaves: usize) -> usize {
        // if leaves is 0, then node count is -1, and 1 past the end is 0, it will be
        // detected instantly

        if leaves == 0 {
            return 1;
        }

        2 * leaves - 1
    }

    pub fn build(&mut self, leaves: &mut [InstanceInfo]) {
        let total = self.build_recursive(0, leaves);

        info!("{:?}", self.nodes);

        assert_eq!(total as usize, self.nodes.len())
    }

    fn build_recursive(&mut self, mut offset: u32, leaves: &mut [InstanceInfo]) -> u32 {
        let curr = offset as usize;
        offset += 1; // go to next

        if leaves.is_empty() {
            // if there are no leaves, set the root AABB to just be all zeroes
            // and set the skip to the limit so we bail out instantly

            self.nodes[curr].bmin = [0.0, 0.0, 0.0];
            self.nodes[curr].bmax = [0.0, 0.0, 0.0];
            self.nodes[curr].packed2 = 0xffffffff;
            self.nodes[curr].packed1 = self.nodes.len() as u32;

            return offset;
        }

        info!("leaves = {:?}", leaves);

        let bbox = BoundingBox::from_extents(leaves.iter().map(|i| i.bbox));

        /*

        algorithm to build the roped BVH is simple:

            1. get bbox of all children here
            2. set it on this node
            3. if there's only one child, this is a LEAF node, populate the relevant fields

        */

        self.nodes[curr].bmin = bbox.min.into();
        self.nodes[curr].bmax = bbox.max.into();

        if leaves.len() == 1 {
            let leaf = &leaves[0];

            // NOTE: for leaves, the "skip" pointer is always 1

            /*

            PACKED1 contains:

                lower 16 bits: the "geometry" ID, i.e. leaf.geometry
                upper 16 bits: the "instance" ID, i.e. leaf geo_start

            PACKED2 contains:

                lower 16 bits: the "material" ID, i.e. leaf.material
                upper 16 bits: the "mat_inst" ID, i.e. leaf.mat_start

            */

            self.nodes[curr].packed2 = leaf.material as u32 | ((leaf.mat_start as u32) << 16);
            self.nodes[curr].packed1 = leaf.geometry as u32 | ((leaf.geo_start as u32) << 16);

            return offset;
        }

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

        let lhs_offset = self.build_recursive(offset, lhs);
        let rhs_offset = self.build_recursive(lhs_offset, rhs);

        // for nodes, packed2 is always zero and packed1 just contains the skip

        self.nodes[curr].packed2 = 0xffffffff;
        self.nodes[curr].packed1 = rhs_offset; // is this +1 ?? or -1?

        rhs_offset
    }
}

impl ToDevice<[SceneHierarchyNode]> for InstancesWithObjects<'_> {
    fn to_device(&self, memory: &mut [SceneHierarchyNode]) {
        let mut instances = Vec::with_capacity(self.instances.list.len());
        let mut geometry_start = 0;
        let mut material_start = 0;

        for (index, instance) in self.instances.list.iter().enumerate() {
            let object = &self.objects[instance.geometry];

            // TODO: handle errors gracefully here somehow? it would indicate bad data
            let bbox = object.bounding_box(&instance.geometry_values).unwrap();

            // need to divide all starts by 4 because we use vec4 buffers...
            // (note: this is an implementation detail)

            instances.push(InstanceInfo {
                bbox,
                surface_area: 1.0, // obtain from the geometry somehow (at least an approximation)
                geometry: instance.geometry as u16,
                material: instance.material as u16,
                geo_start: geometry_start / 4, /* how to obtain? for now we can sum up the
                                                * parameter
                                                * list */
                mat_start: material_start / 4,
            });

            geometry_start += instance.geometry_values.len() as u16;
            material_start += instance.material_values.len() as u16;
        }

        let node_count = SceneHierarchyBuilder::node_count_for_leaves(self.instances.list.len());

        SceneHierarchyBuilder::new(&mut memory[..node_count]).build(&mut instances);
    }

    fn requested_count(&self) -> usize {
        SceneHierarchyBuilder::node_count_for_leaves(self.instances.list.len())
    }
}
/*

impl ToDevice<[InstanceData]> for InstancesWithObjects<'_> {
    fn to_device(&self, slice: &mut [InstanceData]) {
        let indices = Self::calculate_indices(&self.objects);
        let mut material_offset = 0; // this is per-instance

        for (memory, instance) in izip!(&mut *slice, &self.instances.list) {
            let instance_world = Decomposed {
                scale: instance.scale,
                rot: instance.rotation,
                disp: instance.translation,
            };

            if let Some(world_instance) = instance_world.inverse_transform() {
                Self::pack_xfm_row_major(world_instance.into(), &mut memory.transform);
            } else {
                panic!("instance has a non-invertible affine transform (is scale zero?)");
            }

            // TODO: add indexing checks here... or use Rc or something?

            // None of the static geometry offsets depend on the instance itself, we just
            // always duplicate them for every instance since we have the space for them.

            let index_data = &indices[instance.object];

            memory.accel_root_node = index_data.accel_root_node;
            memory.topology_offset = index_data.topology_offset;
            memory.geometry_offset = index_data.geometry_offset;
            memory.material_offset = material_offset;

            // We always store a one-to-one mapping between instance materials and object
            // materials inside the lookup array (which might point to shared materials).

            if instance.materials.len() != self.objects[instance.object].materials {
                panic!("one-to-one mapping required between instance & object materials");
            }

            material_offset += self.objects[instance.object].materials as u32;
        }
    }

    fn requested_count(&self) -> usize {
        self.instances.list.len()
    }
}

impl ToDevice<[MaterialIndex]> for InstancesWithObjects<'_> {
    fn to_device(&self, slice: &mut [MaterialIndex]) {
        let mut index = 0;

        for instance in &self.instances.list {
            for &material in &instance.materials {
                // 16-byte alignment...
                slice[index] = MaterialIndex([material as u32, 0, 0, 0]);

                index += 1;
            }
        }
    }

    fn requested_count(&self) -> usize {
        self.instances
            .list
            .iter()
            .map(|inst| inst.materials.len())
            .sum()
    }
}

*/

impl InstancesWithObjects<'_> {
    /*fn pack_xfm_row_major(xfm: cgmath::Matrix4<f32>, output: &mut [f32; 12]) {
        for (i, j) in iproduct!(0..4, 0..3) {
            output[4 * j + i] = xfm[i][j];
        }
    }*/
}

pub struct InstancesWithObjects<'a> {
    pub instances: &'a Instances,
    pub objects: &'a [Geometry],
}

// We will need the GLSL code for the primitive of course, but also the list of
// symbolic values which can just go into some UBO array as an array of f32. We
// need the GLSL code to be generated to reference these indices efficiently

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct GeometryParameter(f32);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct MaterialParameter(f32);

impl ToDevice<[GeometryParameter]> for InstancesWithObjects<'_> {
    fn to_device(&self, slice: &mut [GeometryParameter]) {
        let mut index = 0;

        for instance in &self.instances.list {
            for &value in &instance.geometry_values {
                slice[index] = GeometryParameter(value);

                index += 1;
            }
        }
    }

    fn requested_count(&self) -> usize {
        self.instances
            .list
            .iter()
            .map(|inst| inst.geometry_values.len())
            .sum()
    }
}

impl ToDevice<[MaterialParameter]> for InstancesWithObjects<'_> {
    fn to_device(&self, slice: &mut [MaterialParameter]) {
        let mut index = 0;

        for instance in &self.instances.list {
            for &value in &instance.material_values {
                slice[index] = MaterialParameter(value);

                index += 1;
            }
        }
    }

    fn requested_count(&self) -> usize {
        self.instances
            .list
            .iter()
            .map(|inst| inst.material_values.len())
            .sum()
    }
}
