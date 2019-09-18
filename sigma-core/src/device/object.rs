use crate::device::ToDevice;
use crate::model::Objects;
use zerocopy::{AsBytes, FromBytes};

// TODO: give these proper types later on? atm we just treat them as byte blobs

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct HierarchyData(u8);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct TriangleData(u8);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct VertexPositionData(u8);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct VertexMappingData(u8);

// TODO: might be good to check for overflow here and stuff

impl ToDevice<[HierarchyData]> for Objects {
    fn to_device(&self, slice: &mut [HierarchyData]) {
        let mut bytes = slice.as_bytes_mut();

        for object in &self.list {
            let (region, rest) = bytes.split_at_mut(object.hierarchy.len());
            region.copy_from_slice(&object.hierarchy);
            bytes = rest;
        }
    }

    fn requested_count(&self) -> usize {
        self.list.iter().map(|obj| obj.hierarchy.len()).sum()
    }
}

impl ToDevice<[TriangleData]> for Objects {
    fn to_device(&self, slice: &mut [TriangleData]) {
        let mut bytes = slice.as_bytes_mut();

        for object in &self.list {
            let (region, rest) = bytes.split_at_mut(object.triangles.len());
            region.copy_from_slice(&object.triangles);
            bytes = rest;
        }
    }

    fn requested_count(&self) -> usize {
        self.list.iter().map(|obj| obj.triangles.len()).sum()
    }
}

impl ToDevice<[VertexPositionData]> for Objects {
    fn to_device(&self, slice: &mut [VertexPositionData]) {
        let mut bytes = slice.as_bytes_mut();

        for object in &self.list {
            let (region, rest) = bytes.split_at_mut(object.positions.len());
            region.copy_from_slice(&object.positions);
            bytes = rest;
        }
    }

    fn requested_count(&self) -> usize {
        self.list.iter().map(|obj| obj.positions.len()).sum()
    }
}

impl ToDevice<[VertexMappingData]> for Objects {
    fn to_device(&self, slice: &mut [VertexMappingData]) {
        let mut bytes = slice.as_bytes_mut();

        for object in &self.list {
            let (region, rest) = bytes.split_at_mut(object.normal_tangent_uv.len());
            region.copy_from_slice(&object.normal_tangent_uv);
            bytes = rest;
        }
    }

    fn requested_count(&self) -> usize {
        self.list
            .iter()
            .map(|obj| obj.normal_tangent_uv.len())
            .sum()
    }
}
