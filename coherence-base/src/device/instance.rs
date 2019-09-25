#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::device::ToDevice;
use crate::model::{Geometry, Instances};
use crate::BoundingBox;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(32), C)]
#[derive(AsBytes, FromBytes, Debug, Default)]
pub struct SceneInstanceNode {
    min: [f32; 3],
    packed1: u32, // "skip" pointer + "parameter offset" << 16
    max: [f32; 3],
    packed2: u32, // geometry ID + material ID << 16  == 0 if it's not a leaf
}

// TODO: check bits 15 are never used for any u16 here because we rely on some
// assumptions

impl SceneInstanceNode {
    pub fn make_leaf(
        min: [f32; 3],
        max: [f32; 3],
        geometry: u16,
        geo_data: u16,
        material: u16,
        mat_data: u16,
        is_last: bool,
    ) -> Self {
        Self {
            min,
            max,
            packed1: Self::pack_u32(geo_data, geometry) | Self::is_last_bit(is_last),
            packed2: Self::pack_u32(mat_data, material),
        }
    }

    pub fn make_node(min: [f32; 3], max: [f32; 3], skip_val: u16) -> Self {
        Self {
            min,
            max,
            packed1: skip_val as u32,
            packed2: 0xffff_ffff,
        }
    }

    fn is_last_bit(is_last: bool) -> u32 {
        if is_last {
            0x8000
        } else {
            0x0000
        }
    }

    fn pack_u32(hi: u16, lo: u16) -> u32 {
        (u32::from(hi) << 16) | u32::from(lo)
    }
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
struct HierarchyBuilder<'a> {
    nodes: &'a mut [SceneInstanceNode],
}

impl<'a> HierarchyBuilder<'a> {
    pub fn new(nodes: &'a mut [SceneInstanceNode]) -> Self {
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
        assert_eq!(total as usize, self.nodes.len())
    }

    fn build_recursive(&mut self, mut offset: u32, leaves: &mut [InstanceInfo]) -> u32 {
        let curr = offset as usize;
        offset += 1; // go to next

        if leaves.is_empty() {
            // if there are no leaves, set the root AABB to just be all zeroes
            // and set the skip to the limit so we bail out instantly

            self.nodes[curr] = SceneInstanceNode::make_node([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0);

            return offset;
        }

        let bbox = BoundingBox::from_extents(leaves.iter().map(|i| i.bbox));

        /*

        algorithm to build the roped BVH is simple:

            1. get bbox of all children here
            2. set it on this node
            3. if there's only one child, this is a LEAF node, populate the relevant fields

        */

        if leaves.len() == 1 {
            let leaf = &leaves[0];

            self.nodes[curr] = SceneInstanceNode::make_leaf(
                bbox.min.into(),
                bbox.max.into(),
                leaf.geometry,
                leaf.geo_start,
                leaf.material,
                leaf.mat_start,
                offset as usize == self.nodes.len(),
            );

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

        self.nodes[curr] = SceneInstanceNode::make_node(
            bbox.min.into(),
            bbox.max.into(),
            (rhs_offset as u16) % (self.nodes.len() as u16),
        );

        rhs_offset
    }
}

impl ToDevice<[SceneInstanceNode]> for InstancesWithObjects<'_> {
    fn to_device(&self, memory: &mut [SceneInstanceNode]) {
        let mut instances = Vec::with_capacity(self.instances.list.len());
        let mut geometry_start = 0;
        let mut material_start = 0;

        for instance in &self.instances.list {
            let object = &self.objects[instance.geometry];

            // TODO: handle errors gracefully here somehow? it would indicate bad data
            let bbox = object.bounding_box(&instance.geometry_values).unwrap();

            instances.push(InstanceInfo {
                bbox,
                surface_area: 1.0, // obtain from the geometry somehow (at least an approximation)
                geometry: instance.geometry as u16,
                material: instance.material as u16,
                geo_start: geometry_start,
                mat_start: material_start / 4,
            });

            // need to divide all starts by 4 because we use vec4 buffers...
            // (note: this is an implementation detail)

            // in this case we KNOW there are only that many, so we don't really need to do
            // renumbering here; we just take the total number of values and go with that

            geometry_start += (instance.geometry_values.len() as u16) / 4;
            material_start += instance.material_values.len() as u16;
        }

        let node_count = HierarchyBuilder::node_count_for_leaves(self.instances.list.len());

        HierarchyBuilder::new(&mut memory[..node_count]).build(&mut instances);
    }

    fn requested_count(&self) -> usize {
        HierarchyBuilder::node_count_for_leaves(self.instances.list.len())
    }
}

pub struct InstancesWithObjects<'a> {
    pub instances: &'a Instances,
    pub objects: &'a [Geometry],
}

// We will need the GLSL code for the primitive of course, but also the list of
// symbolic values which can just go into some UBO array as an array of f32. We
// need the GLSL code to be generated to reference these indices efficiently

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes)]
pub struct GeometryParameter([f32; 4]);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct MaterialParameter(f32);

use itertools::izip;

impl ToDevice<[GeometryParameter]> for InstancesWithObjects<'_> {
    fn to_device(&self, mut slice: &mut [GeometryParameter]) {
        // This implements parameter renumbering to ensure that all memory accesses in
        // the parameter array are coherent and that all fields are nicely packed into
        // individual vec4 elements. Out-of-bounds parameter indices are checked here.

        for instance in &self.instances.list {
            let indices = self.objects[instance.geometry].symbolic_parameter_indices();
            let (region, remaining_data) = slice.split_at_mut((indices.len() + 3) / 4);

            for (data, indices) in izip!(region, indices.chunks(4)) {
                for i in 0..4 {
                    if let Some(&index) = indices.get(i) {
                        data.0[i] = instance.geometry_values[index];
                    } else {
                        data.0[i] = 0.0; // unused (for vec4 padding)
                    }
                }
            }

            slice = remaining_data;
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
