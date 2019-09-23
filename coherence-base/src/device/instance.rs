#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::device::ToDevice;
use crate::model::{Instances, Object};
use crate::BoundingBox;
use cgmath::prelude::*;
use cgmath::Decomposed;
use itertools::{iproduct, izip};
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Clone, Copy, FromBytes, AsBytes)]
pub struct InstanceData {
    /// Row-major matrix representation of the instance's affine transformation.
    transform: [f32; 12],
    /// Index of this instance's root node in the acceleration structure array.
    accel_root_node: u32,
    /// Absolute offset into the per-face data array for this instance.
    topology_offset: u32,
    /// Absolute offset into the per-vertex data array for this instance.
    geometry_offset: u32,
    /// Absolute offset into the material lookup array for this instance.
    material_offset: u32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct IndexData {
    accel_root_node: u32,
    topology_offset: u32,
    geometry_offset: u32,
}

#[repr(align(64), C)]
#[derive(AsBytes, FromBytes)]
pub struct SceneHierarchyNode {
    lhs_bmin: [f32; 3],
    lhs_next: u32,
    lhs_bmax: [f32; 3],
    lhs_inst: u32,
    rhs_bmin: [f32; 3],
    rhs_next: u32,
    rhs_bmax: [f32; 3],
    rhs_inst: u32,
}

struct InstanceInfo {
    bbox: BoundingBox,
    scale: f32,
    surface_area: f32,
    inst: u32,
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

pub struct InstancesWithObjects<'a> {
    pub instances: &'a Instances,
    pub objects: &'a [Object],
}

impl ToDevice<[SceneHierarchyNode]> for InstancesWithObjects<'_> {
    fn to_device(&self, memory: &mut [SceneHierarchyNode]) {
        let mut instances = Vec::with_capacity(self.instances.list.len());

        for (index, instance) in self.instances.list.iter().enumerate() {
            let object = &self.objects[instance.object];

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

        let node_count = SceneHierarchyBuilder::node_count_for_leaves(self.instances.list.len());

        SceneHierarchyBuilder::new(&mut memory[..node_count]).build(&mut instances);
    }

    fn requested_count(&self) -> usize {
        SceneHierarchyBuilder::node_count_for_leaves(self.instances.list.len())
    }
}

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

impl InstancesWithObjects<'_> {
    fn calculate_indices(objects: &[Object]) -> Vec<IndexData> {
        let indices = objects.iter().scan(IndexData::default(), |state, obj| {
            let current = *state;

            state.accel_root_node += obj.hierarchy.len() as u32 / 64;
            state.topology_offset += obj.triangles.len() as u32 / 64;
            state.geometry_offset += obj.positions.len() as u32 / 16;

            Some(current)
        });

        indices.collect()
    }

    fn pack_xfm_row_major(xfm: cgmath::Matrix4<f32>, output: &mut [f32; 12]) {
        for (i, j) in iproduct!(0..4, 0..3) {
            output[4 * j + i] = xfm[i][j];
        }
    }
}
