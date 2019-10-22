#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::renumber_parameters;
use crate::BoundingBox;
use crate::Device;
use crate::{material_index, material_parameter_block_count};
use crate::{Geometry, Instance, Material};
use itertools::izip;
use js_sys::Error;
use std::cmp::Ordering;
use std::mem::swap;
use zerocopy::{AsBytes, FromBytes};

impl Device {
    pub(crate) fn update_instances(
        &mut self,
        geometry_list: &[Geometry],
        material_list: &[Material],
        instance_list: &[Instance],
    ) -> Result<(), Error> {
        // update the instance BVH

        let mut material_start = vec![];
        let mut count = 0;

        for material in material_list {
            material_start.push(count);

            count += material_parameter_block_count(material) as u16;
        }

        let mut instance_info = Vec::with_capacity(instance_list.len());
        let mut geometry_start = 0;

        for instance in instance_list {
            let geometry = &geometry_list[instance.geometry];
            let material = &material_list[instance.material];

            let bbox = geometry
                .bounding_box(&instance.parameters)
                .ok_or_else(|| Error::new("bad instance parameters"))?;

            instance_info.push(InstanceInfo {
                bbox,
                cost: geometry.evaluation_cost(),
                allow_mis: instance.allow_mis,
                geometry: instance.geometry as u16,
                geo_inst: geometry_start,
                material: material_index(material),
                mat_inst: material_start[instance.material],
            });

            geometry_start += (instance.parameters.len() as u16 + 3) / 4;
        }

        let node_count = HierarchyBuilder::node_count_for_leaves(instance_list.len());

        let mut nodes = self.allocator.allocate(node_count);

        HierarchyBuilder::new(&mut nodes).build(&mut instance_info);

        self.instance_buffer.write_array(&nodes)?;
        self.program
            .set_define("INSTANCE_DATA_COUNT", self.instance_buffer.element_count());

        if instance_info.is_empty() {
            self.program.set_define("INSTANCE_DATA_PRESENT", 0);
        } else {
            self.program.set_define("INSTANCE_DATA_PRESENT", 1);
        }

        // This implements parameter renumbering to ensure that all memory accesses in
        // the parameter array are coherent and that all fields are nicely packed into
        // individual vec4 elements. Out-of-bounds parameter indices are checked here.

        let geometry_parameter_count: usize = instance_list
            .iter()
            .map(|inst| (inst.parameters.len() + 3) / 4)
            .sum();

        let params: &mut [GeometryParameter] = self.allocator.allocate(geometry_parameter_count);
        let mut offset = 0;

        for instance in instance_list {
            let indices = renumber_parameters(&geometry_list[instance.geometry]);
            let block_count = (indices.len() + 3) / 4;

            let region = &mut params[offset..offset + block_count];
            offset += block_count;

            for (data, indices) in izip!(region, indices.chunks(4)) {
                for i in 0..4 {
                    if let Some(&index) = indices.get(i) {
                        data.0[i] = instance.parameters[index];
                    } else {
                        data.0[i] = 0.0; // unused vec4 padding
                    }
                }
            }
        }

        self.geometry_buffer.write_array(&params)?;
        self.program
            .set_define("GEOMETRY_DATA_COUNT", self.geometry_buffer.element_count());

        Ok(())
    }
}

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct GeometryParameter([f32; 4]);

#[repr(align(32), C)]
#[derive(AsBytes, FromBytes, Debug, Default)]
pub struct SceneInstanceNode {
    min: [f32; 3],
    word1: u32,
    max: [f32; 3],
    word2: u32,
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
        allow_mis: bool,
    ) -> Self {
        assert!(geometry < 0x8000 && geo_inst < 0x8000);
        assert!(material < 0x8000 && mat_inst < 0x8000);

        Self {
            min,
            max,
            word1: Self::pack_u32(geo_inst, geometry) | Self::is_last_bit(is_last),
            word2: Self::pack_u32(mat_inst, material) | Self::mis_flag(allow_mis),
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

    pub fn make_root() -> Self {
        Self {
            min: [0.0; 3],
            max: [0.0; 3],
            word1: 0x0000_8000,
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

    fn mis_flag(allow_mis: bool) -> u32 {
        if allow_mis {
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
    pub cost: f32,
    pub allow_mis: bool,
    pub geometry: u16,
    pub geo_inst: u16,
    pub material: u16,
    pub mat_inst: u16,
}

#[derive(Clone, Copy, Debug)]
struct CandidateSplit {
    pub axis: usize,
    pub pos: usize,
    pub swap: bool,
    pub cost: f32,
}

impl CandidateSplit {
    pub fn initial() -> Self {
        Self {
            axis: 0,
            pos: 0,
            swap: false,
            cost: std::f32::INFINITY,
        }
    }
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
        2 * leaves.max(1) - 1
    }

    pub fn build(&mut self, leaves: &mut [InstanceInfo]) {
        assert_eq!(self.build_recursive(0, leaves), 0);
    }

    fn build_recursive(&mut self, offset: u32, leaves: &mut [InstanceInfo]) -> u32 {
        match leaves {
            [] => self.build_recursive_empty(),
            [leaf] => self.build_recursive_one(offset, leaf),
            leaves => self.build_recursive_many(offset, leaves),
        }
    }

    fn build_recursive_empty(&mut self) -> u32 {
        self.nodes[0] = SceneInstanceNode::make_root();
        0 // there will be a root node on an empty BVH
    }

    fn build_recursive_one(&mut self, offset: u32, leaf: &InstanceInfo) -> u32 {
        let is_last = offset == self.nodes.len() as u32 - 1;

        self.nodes[offset as usize] = SceneInstanceNode::make_leaf(
            leaf.bbox.min.into(),
            leaf.bbox.max.into(),
            leaf.geometry,
            leaf.geo_inst,
            leaf.material,
            leaf.mat_inst,
            is_last,
            leaf.allow_mis,
        );

        if !is_last {
            offset + 1
        } else {
            0
        }
    }

    fn build_recursive_many(&mut self, mut offset: u32, leaves: &mut [InstanceInfo]) -> u32 {
        let mut best_split = CandidateSplit::initial();

        for axis in 0..3 {
            Self::sort_by_leaf_centroid_on_axis(leaves, axis);

            for pos in 1..leaves.len() {
                let mut lhs_bbox = BoundingBox::neg_infinity_bounds();
                let mut rhs_bbox = BoundingBox::neg_infinity_bounds();
                let mut lhs_cost = 0.0;
                let mut rhs_cost = 0.0;

                for (i, leaf) in leaves.iter().enumerate() {
                    let weighted_cost = leaf.bbox.surface_area() * leaf.cost;

                    if i < pos {
                        lhs_bbox.extend(&leaf.bbox);
                        lhs_cost += weighted_cost;
                    } else {
                        rhs_bbox.extend(&leaf.bbox);
                        rhs_cost += weighted_cost;
                    }
                }

                let lhs_area = lhs_bbox.surface_area();
                let rhs_area = rhs_bbox.surface_area();

                lhs_cost *= lhs_area;
                rhs_cost *= rhs_area;

                let swap = lhs_area < rhs_area;
                let cost = lhs_cost + rhs_cost;

                if cost < best_split.cost {
                    best_split = CandidateSplit {
                        axis,
                        pos,
                        swap,
                        cost,
                    };
                }
            }
        }

        assert!(best_split.cost.is_finite());

        Self::sort_by_leaf_centroid_on_axis(leaves, best_split.axis);
        let (mut lhs, mut rhs) = leaves.split_at_mut(best_split.pos);

        // Keep the node with the largest surface area on the left, this promotes faster
        // traversal because the current algorithm always traverses the left node first.

        if best_split.swap {
            swap(&mut lhs, &mut rhs);
        }

        let current = offset as usize;
        offset += 1; // allocate node

        offset = self.build_recursive(offset, lhs);
        offset = self.build_recursive(offset, rhs);

        if offset == self.nodes.len() as u32 {
            offset = 0; // final node in BVH
        }

        let mut bbox = BoundingBox::neg_infinity_bounds();

        for leaf in leaves {
            bbox.extend(&leaf.bbox);
        }

        self.nodes[current] =
            SceneInstanceNode::make_node(bbox.min.into(), bbox.max.into(), offset);

        offset
    }

    fn sort_by_leaf_centroid_on_axis(leaves: &mut [InstanceInfo], axis: usize) {
        leaves.sort_unstable_by(|lhs, rhs| {
            let lhs_centroid = lhs.bbox.max[axis] - lhs.bbox.min[axis];
            let rhs_centroid = rhs.bbox.max[axis] - rhs.bbox.min[axis];

            let ordering = lhs_centroid.partial_cmp(&rhs_centroid);
            ordering.unwrap_or(Ordering::Less) // we assume no NaNs
        });
    }
}
