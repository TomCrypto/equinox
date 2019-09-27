#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::render::renumber_parameters;
use crate::Device;
use coherence_base::{Geometry, HierarchyBuilder, InstanceInfo, Instances};
use itertools::izip;
use zerocopy::{AsBytes, FromBytes};

#[repr(transparent)]
#[derive(AsBytes, FromBytes, Copy, Clone)]
pub struct MaterialIndex([u32; 4]);

impl Device {
    pub(crate) fn update_instances(&mut self, objects: &[Geometry], instances: &Instances) {
        // update the instance BVH

        let mut instance_info = Vec::with_capacity(instances.list.len());
        let mut geometry_start = 0;
        let mut material_start = 0;

        for instance in &instances.list {
            let object = &objects[instance.geometry];

            // TODO: handle errors gracefully here somehow? it would indicate bad data
            let bbox = object.bounding_box(&instance.geometry_values).unwrap();

            instance_info.push(InstanceInfo {
                bbox,
                surface_area: 1.0, // obtain from the geometry somehow (at least an approximation)
                geometry: instance.geometry as u16,
                material: instance.material as u16,
                geo_inst: geometry_start,
                mat_inst: material_start / 4,
            });

            // need to divide all starts by 4 because we use vec4 buffers...
            // (note: this is an implementation detail)

            // in this case we KNOW there are only that many, so we don't really need to do
            // renumbering here; we just take the total number of values and go with that

            geometry_start += (instance.geometry_values.len() as u16) / 4;
            material_start += instance.material_values.len() as u16;
        }

        let node_count = HierarchyBuilder::node_count_for_leaves(instances.list.len());

        let mut nodes = self.scratch.allocate(node_count);

        HierarchyBuilder::new(&mut nodes).build(&mut instance_info);

        self.instance_buffer.write_array(&nodes);

        // update the geometry data

        // This implements parameter renumbering to ensure that all memory accesses in
        // the parameter array are coherent and that all fields are nicely packed into
        // individual vec4 elements. Out-of-bounds parameter indices are checked here.

        let geometry_parameter_count: usize = instances
            .list
            .iter()
            .map(|inst| inst.geometry_values.len())
            .sum();

        let mut params: &mut [GeometryParameter] =
            self.scratch.allocate(geometry_parameter_count / 4);

        for instance in &instances.list {
            let indices = renumber_parameters(&objects[instance.geometry]);
            let (region, remaining_data) = params.split_at_mut((indices.len() + 3) / 4);

            for (data, indices) in izip!(region, indices.chunks(4)) {
                for i in 0..4 {
                    if let Some(&index) = indices.get(i) {
                        data.0[i] = instance.geometry_values[index];
                    } else {
                        data.0[i] = 0.0; // unused (for vec4 padding)
                    }
                }
            }

            params = remaining_data;
        }

        self.geometry_buffer.write_array(&params);

        // update the material data

        let material_parameter_count: usize = instances
            .list
            .iter()
            .map(|inst| inst.material_values.len())
            .sum();

        let material_params = self.scratch.allocate(material_parameter_count);

        let mut index = 0;

        for instance in &instances.list {
            for &value in &instance.material_values {
                material_params[index] = MaterialParameter(value);

                index += 1;
            }
        }

        self.material_buffer.write_array(&material_params);
    }
}

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes)]
pub struct GeometryParameter([f32; 4]);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct MaterialParameter(f32);
