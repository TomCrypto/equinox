use crate::device::ToDevice;
use crate::model::EnvironmentMap;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(FromBytes, AsBytes)]
pub struct EnvironmentMapCdfData([f32; 4]);

#[repr(C)]
#[derive(FromBytes, AsBytes)]
pub struct EnvironmentMapPixelData([f32; 4]);

// TODO: separate rows and columns later on for possibly faster inverse
// transform sampling and less memory usage and is just better
// TODO: the ToDevice system is stupid here as we need to do a lot more work

impl ToDevice<[EnvironmentMapPixelData]> for EnvironmentMap {
    fn to_device(&self, slice: &mut [EnvironmentMapPixelData]) {
        for y in 0..self.height {
            for x in 0..self.width {
                let i = (y * self.width + x) as usize;

                let r = self.pixels[3 * (y * self.width + x) as usize];
                let g = self.pixels[3 * (y * self.width + x) as usize + 1];
                let b = self.pixels[3 * (y * self.width + x) as usize + 2];

                // (u, v, cdf, 0.0)

                slice[i].0[0] = r;
                slice[i].0[1] = g;
                slice[i].0[2] = b;
                slice[i].0[3] = 0.0;
            }
        }
    }

    fn requested_count(&self) -> usize {
        (self.height as usize) * (self.width as usize)
    }
}

impl ToDevice<[EnvironmentMapCdfData]> for EnvironmentMap {
    fn to_device(&self, slice: &mut [EnvironmentMapCdfData]) {
        let mut cdf = vec![];

        let mut running = 0.0;

        for y in 0..self.height {
            for x in 0..self.width {
                let r = self.pixels[3 * (y * self.width + x) as usize];
                let g = self.pixels[3 * (y * self.width + x) as usize + 1];
                let b = self.pixels[3 * (y * self.width + x) as usize + 2];

                let avg = (r + g + b) / 3.0; // TODO: weigh by sin(theta)?

                cdf.push(running);

                running += avg;
            }
        }

        cdf.push(running);

        for i in 0..cdf.len() {
            cdf[i] /= running;
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let i = (y * self.width + x) as usize;

                // (u, v, cdf, 0.0)

                slice[i].0[0] = (x as f32) / ((self.width - 1) as f32);
                slice[i].0[1] = (y as f32) / ((self.height - 1) as f32);
                slice[i].0[2] = cdf[i];
                slice[i].0[3] = 0.0;
            }
        }

        /*

        1. just allocate a big-ass buffer and store the normalized CDF in there
            -> remember to weigh by the sin(theta) term here!
        2. the data in each is of the form

        sampling algorithm:

         - pick a random uniform u in [0, 1]
         - search the location in the CDF with that u
         - this will correspond to 1 pixel in the image; store (u, v) coords as half-floats
           and recover the direction in the shader

        */

        // ...
    }

    fn requested_count(&self) -> usize {
        (self.height as usize) * (self.width as usize) + 1
    }
}
