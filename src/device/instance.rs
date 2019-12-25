#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{
    material_index, material_parameter_count, BoundingBox, Device, Geometry, Instance, Material,
};
use itertools::izip;
use js_sys::Error;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::mem::swap;
use zerocopy::{AsBytes, FromBytes};

impl Device {
    pub(crate) fn update_instances(
        &mut self,
        geometry_list: &BTreeMap<String, Geometry>,
        material_list: &BTreeMap<String, Material>,
        instance_list: &BTreeMap<String, Instance>,
    ) -> Result<(), Error> {
        // update the instance BVH

        let mut material_start = BTreeMap::new();
        let mut geometry_index = BTreeMap::new();
        let mut count = 0;

        for (name, material) in material_list {
            material_start.insert(name.to_owned(), count);

            count += material_parameter_count(material) as u16;
        }

        for (index, name) in geometry_list.keys().enumerate() {
            geometry_index.insert(name.to_owned(), index);
        }

        let mut instance_info = Vec::with_capacity(instance_list.len());
        let mut geometry_start = 0;

        for instance in instance_list.values() {
            if !instance.visible {
                continue;
            }

            let geometry = &geometry_list[&instance.geometry];
            let material = &material_list[&instance.material];

            let bbox = geometry.bounding_box(&instance.parameters);

            instance_info.push(InstanceInfo {
                bbox,
                cost: geometry.evaluation_cost(),
                photon_receiver: instance.photon_receiver && !material.has_delta_bsdf(),
                sample_explicit: instance.sample_explicit && !material.has_delta_bsdf(),
                geometry: geometry_index[&instance.geometry] as u16,
                geo_inst: geometry_start,
                material: material_index(material),
                mat_inst: material_start[&instance.material],
            });

            geometry_start += 2 + (instance.parameters.len() as u16 + 3) / 4;
        }

        let node_count = HierarchyBuilder::node_count_for_leaves(instance_info.len());

        let mut nodes = vec![SceneInstanceNode::default(); node_count];

        HierarchyBuilder::new(&mut nodes).build(&mut instance_info);

        self.instance_buffer
            .write_array(self.instance_buffer.max_len(), &nodes)?;
        self.integrator_gather_photons_shader
            .set_define("INSTANCE_DATA_LEN", self.instance_buffer.len());
        self.integrator_scatter_photons_shader
            .set_define("INSTANCE_DATA_LEN", self.instance_buffer.len());

        if instance_info.is_empty() {
            self.integrator_gather_photons_shader
                .set_define("INSTANCE_DATA_PRESENT", 0);
            self.integrator_scatter_photons_shader
                .set_define("INSTANCE_DATA_PRESENT", 0);
        } else {
            self.integrator_gather_photons_shader
                .set_define("INSTANCE_DATA_PRESENT", 1);
            self.integrator_scatter_photons_shader
                .set_define("INSTANCE_DATA_PRESENT", 1);
        }

        // This implements parameter renumbering to ensure that all memory accesses in
        // the parameter table are coherent and that all fields are nicely packed into
        // individual vec4 elements. Out-of-bounds parameter indices are checked here.

        let mut params = vec![];
        let mut offset = 0;

        for instance in instance_list.values() {
            if !instance.visible {
                continue;
            }

            if let Some(parent) = &instance.parent {
                let parent = &instance_list[parent];

                params.push(GeometryParamData([
                    parent.medium.extinction[0],
                    parent.medium.extinction[1],
                    parent.medium.extinction[2],
                    parent.medium.refractive_index,
                ]));
            } else {
                params.push(GeometryParamData([0.0, 0.0, 0.0, 1.0]));
            }

            params.push(GeometryParamData([
                instance.medium.extinction[0],
                instance.medium.extinction[1],
                instance.medium.extinction[2],
                instance.medium.refractive_index,
            ]));

            offset += 2;

            let parameters = geometry_list[&instance.geometry].symbolic_parameters();
            let block_count = (parameters.len() + 3) / 4;

            for _ in 0..block_count {
                params.push(GeometryParamData::default());
            }

            let region = &mut params[offset..offset + block_count];
            offset += block_count;

            for (data, parameters) in izip!(region, parameters.chunks(4)) {
                for i in 0..4 {
                    if let Some(&symbol) = parameters.get(i) {
                        data.0[i] = instance.parameters[symbol];
                    } else {
                        data.0[i] = 0.0; // unused vec4 padding
                    }
                }
            }
        }

        self.geometry_buffer
            .write_array(self.geometry_buffer.max_len(), &params)?;
        self.integrator_gather_photons_shader
            .set_define("GEOMETRY_DATA_LEN", self.geometry_buffer.len());
        self.integrator_scatter_photons_shader
            .set_define("GEOMETRY_DATA_LEN", self.geometry_buffer.len());

        Ok(())
    }
}

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Clone, Copy, Debug, Default)]
pub struct GeometryParamData([f32; 4]);

#[repr(align(32), C)]
#[derive(AsBytes, FromBytes, Clone, Copy, Debug, Default)]
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
        photon_receiver: bool,
        sample_explicit: bool,
    ) -> Self {
        assert!(geometry < 0x8000 && geo_inst < 0x8000);
        assert!(material < 0x8000 && mat_inst < 0x8000);

        let mut material_instance_flags = 0;

        material_instance_flags |= Self::photon_receiver_flag(photon_receiver);
        material_instance_flags |= Self::sample_explicit_flag(sample_explicit);

        let mut geometry_instance_flags = 0;

        geometry_instance_flags |= Self::is_last_flag(is_last);

        Self {
            min,
            max,
            word1: Self::pack_u32(geo_inst, geometry) | geometry_instance_flags,
            word2: Self::pack_u32(mat_inst, material) | material_instance_flags,
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

    fn is_last_flag(is_last: bool) -> u32 {
        if is_last {
            0x8000
        } else {
            0x0000
        }
    }

    fn photon_receiver_flag(photon_receiver: bool) -> u32 {
        if photon_receiver {
            0x8000
        } else {
            0x0000
        }
    }

    fn sample_explicit_flag(sample_explicit: bool) -> u32 {
        if sample_explicit {
            0x4000
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
    pub photon_receiver: bool,
    pub sample_explicit: bool,
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
            leaf.photon_receiver,
            leaf.sample_explicit,
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
