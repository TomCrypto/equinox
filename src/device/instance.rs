#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::renumber_parameters;
use crate::BoundingBox;
use crate::Device;
use crate::{material_index, material_parameter_block_count};
use crate::{Geometry, Instance, Material};
use itertools::izip;
use zerocopy::{AsBytes, FromBytes};

impl Device {
    pub(crate) fn update_instances(
        &mut self,
        geometry_list: &[Geometry],
        material_list: &[Material],
        instance_list: &[Instance],
    ) {
        // update the instance BVH

        let mut material_starts = vec![];
        let mut count = 0;

        for material in material_list {
            material_starts.push(count);

            count += material_parameter_block_count(material) as u16;
        }

        let mut instance_info = Vec::with_capacity(instance_list.len());
        let mut geometry_start = 0;

        for instance in instance_list {
            let geometry = &geometry_list[instance.geometry];
            let material = &material_list[instance.material];

            // TODO: handle errors gracefully here somehow? it would indicate bad data
            let bbox = geometry.bounding_box(&instance.geometry_values).unwrap();

            instance_info.push(InstanceInfo {
                bbox,
                surface_area: 1.0, // obtain from the geometry somehow (at least an approximation)
                geometry: instance.geometry as u16,
                material: material_index(material),
                geo_inst: geometry_start,
                mat_inst: material_starts[instance.material],
            });

            // need to divide all starts by 4 because we use vec4 buffers...
            // (note: this is an implementation detail)

            geometry_start += (instance.geometry_values.len() as u16 + 3) / 4;
        }

        let node_count = HierarchyBuilder::node_count_for_leaves(instance_list.len());

        let mut nodes = self.allocator.allocate(node_count);

        HierarchyBuilder::new(&mut nodes).build(&mut instance_info);

        self.instance_buffer.write_array(&nodes);

        // update the geometry data

        // This implements parameter renumbering to ensure that all memory accesses in
        // the parameter array are coherent and that all fields are nicely packed into
        // individual vec4 elements. Out-of-bounds parameter indices are checked here.

        let geometry_parameter_count: usize = instance_list
            .iter()
            .map(|inst| (inst.geometry_values.len() + 3) / 4)
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
                        data.0[i] = instance.geometry_values[index];
                    } else {
                        data.0[i] = 0.0; // unused (for vec4 padding)
                    }
                }
            }
        }

        self.geometry_buffer.write_array(&params);
    }
}

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct GeometryParameter([f32; 4]);

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
        2 * leaves.max(1) - 1
    }

    pub fn build(&mut self, leaves: &mut [InstanceInfo]) {
        let total = self.build_recursive(0, leaves);
        assert_eq!(total as usize, self.nodes.len())
    }

    fn build_recursive(&mut self, mut offset: u32, leaves: &mut [InstanceInfo]) -> u32 {
        let curr = offset as usize;
        offset += 1; // go to next

        if leaves.is_empty() {
            self.nodes[curr] = SceneInstanceNode::make_node([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0);

            return offset;
        }

        let bbox = BoundingBox::from_extents(leaves.iter().map(|i| i.bbox));

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

        self.nodes[curr] = SceneInstanceNode::make_node(
            bbox.min.into(),
            bbox.max.into(),
            (rhs_offset as u32) % (self.nodes.len() as u32),
        );

        rhs_offset
    }
}
