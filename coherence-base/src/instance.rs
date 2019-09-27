#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::BoundingBox;
use zerocopy::{AsBytes, FromBytes};

#[derive(Default)]
pub struct Instances {
    pub list: Vec<Instance>,
}

// transforms are baked into the SDF nature of the geometry, so it's unnecessary
// to include it here. all we need here is a reference to the geometry, and a
// reference to the material

// what about multiple materials? don't bother for now

pub struct Instance {
    pub geometry: usize,
    pub material: usize,

    pub geometry_values: Vec<f32>,
    pub material_values: Vec<f32>,
}

#[repr(align(32), C)]
#[derive(AsBytes, FromBytes, Debug, Default)]
pub struct SceneInstanceNode {
    min: [f32; 3],
    word1: u32, // "skip" pointer + "parameter offset" << 16
    max: [f32; 3],
    word2: u32, // geometry ID + material ID << 16  == 0 if it's not a leaf
}

impl SceneInstanceNode {
    pub fn make_leaf(
        min: [f32; 3],
        max: [f32; 3],
        geometry: u16,
        geo_inst: u16,
        material: u16,
        mat_inst: u16,
        is_last: bool,
    ) -> Self {
        assert!(geometry < 0x8000);
        assert!(geo_inst < 0x8000);
        assert!(material < 0x8000);
        assert!(mat_inst < 0x8000);

        Self {
            min,
            max,
            word1: Self::pack_u32(geo_inst, geometry) | Self::is_last_bit(is_last),
            word2: Self::pack_u32(mat_inst, material),
        }
    }

    pub fn make_node(min: [f32; 3], max: [f32; 3], skip_val: u32) -> Self {
        Self {
            min,
            max,
            word1: skip_val,
            word2: 0xffff_ffff,
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
pub struct InstanceInfo {
    pub bbox: BoundingBox,
    pub surface_area: f32, // TODO: can we ever actually compute this? might be doable actually
    pub geometry: u16,
    pub material: u16,
    pub geo_inst: u16,
    pub mat_inst: u16,
}

/// Builds an instance BVH for the scene.
pub struct HierarchyBuilder<'a> {
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
                leaf.geo_inst,
                leaf.material,
                leaf.mat_inst,
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

        // for nodes, word2 is always zero and word1 just contains the skip

        self.nodes[curr] = SceneInstanceNode::make_node(
            bbox.min.into(),
            bbox.max.into(),
            (rhs_offset as u32) % (self.nodes.len() as u32),
        );

        rhs_offset
    }
}
