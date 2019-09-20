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
    transform: [f32; 12], // world transform for this instance
    hierarchy_start: u32, // where does the BVH start in the BVH data?
    triangles_start: u32, // where does the triangle data start?
    vertices_start: u32,  // where does the vertex data start?
    materials_start: u32, // where does the material data start? NOT IMPLEMENTED YET
}

#[derive(Clone, Copy, Default, Debug)]
pub struct IndexData {
    hierarchy_start: u32,
    triangles_start: u32,
    materials_start: u32,
    vertices_start: u32,
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

// when do we update the object/material list?
//  -> if it is dirty, it has to have been updated already
// it's public though, but that's the user's fault

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

        for (memory, instance) in izip!(&mut *slice, &self.instances.list) {
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
            memory.triangles_start = index_data.triangles_start;
            memory.vertices_start = index_data.vertices_start;
            memory.materials_start = index_data.materials_start;
        }
    }

    fn requested_count(&self) -> usize {
        self.instances.list.len()
    }
}

impl InstancesWithObjects<'_> {
    fn calculate_indices(objects: &[Object]) -> Vec<IndexData> {
        let mut indices = Vec::with_capacity(objects.len());
        let mut current = IndexData::default();

        for object in objects {
            indices.push(current);

            current.hierarchy_start += object.hierarchy.len() as u32 / 32;
            current.triangles_start += object.triangles.len() as u32 / 16;
            current.vertices_start += object.positions.len() as u32 / 16;
            current.materials_start += object.materials/*.len()*/ as u32;
        }

        indices
    }

    fn pack_xfm_row_major(xfm: cgmath::Matrix4<f32>, output: &mut [f32; 12]) {
        for (i, j) in iproduct!(0..4, 0..3) {
            output[4 * j + i] = xfm[i][j];
        }
    }
}
