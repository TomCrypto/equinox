use std::fmt;
use std::mem::size_of;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

/// Fast device memory allocator.
///
/// This allocator is primarily designed for potentially large, short-lived
/// allocations intended only as a staging area before uploading data up to
/// the device. It uses a watermark system to release memory every frame.
#[derive(Default)]
pub struct Allocator {
    memory: Vec<Aligned>,
    watermark: usize,
}

impl Allocator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shrink_to_watermark(&mut self) {
        self.memory.truncate(self.watermark);
        self.memory.shrink_to_fit(); // free

        self.watermark = 0;
    }

    pub fn allocate<T: AsBytes + FromBytes>(&mut self, len: usize) -> &mut [T] {
        let bytes = self.allocate_bytes(len * size_of::<T>());

        LayoutVerified::new_slice(bytes).unwrap().into_mut_slice()
    }

    pub fn allocate_one<T: AsBytes + FromBytes>(&mut self) -> &mut T {
        &mut self.allocate(1)[0]
    }

    pub fn allocate_bytes(&mut self, len: usize) -> &mut [u8] {
        let blocks = (len + size_of::<Aligned>() - 1) / size_of::<Aligned>();
        self.memory.resize_with(blocks, Aligned::default); // round up length

        self.watermark = self.watermark.max(self.memory.len());
        &mut self.memory.as_mut_slice().as_bytes_mut()[..len]
    }
}

impl fmt::Debug for Allocator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bytes = self.memory.len() * size_of::<Aligned>();
        write!(f, "Allocator {{ {} bytes in use }}", bytes)
    }
}

#[repr(align(64), C)]
#[derive(FromBytes, AsBytes, Clone)]
struct Aligned([u8; 64]);

impl Default for Aligned {
    fn default() -> Self {
        Self([0; 64])
    }
}
