use crate::device::ToDevice;
use crate::model::EnvironmentMap;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(FromBytes, AsBytes)]
pub struct EnvironmentMapCdfData([f32; 4]);

#[repr(C)]
#[derive(FromBytes, AsBytes)]
pub struct EnvironmentMapPixelData([f32; 4]);

/*

We'll precompute this data just so it loads fast, there's no need to compute it on the fly if we
can just avoid it.

Let's say we use a relatively lower-resolution CDF map for memory/performance reasons. Let's say
we sample 8x8 pixels? (otherwise we use a lot of memory: basically two values: the CDF so far
and... what exactly? if we do for each pixel, then what happens? we need to store the CDF for
sampling and as far as I can tell that's all we need?)
 -> two more half floats per texel, roughly? we don't have to bother with the last CDF element of
    value 1, and we can store overall PDF values more intelligently.

Let's just sample 1x1 for now; the rendering code should be pretty agnostic to this anyway; the
envmap would stay the same, just the CDF data changes and becomes a bit smaller; easy.

So, first of all, the HDR data; in the interest of space usage we'll store these as RGBE over the
wire and unpack to RGBA16F (with alpha unused) in the renderer, this is a savings of 2x.
 -> for now we can just use RGBA16F directly

We DO NOT NEED to know the envmap's actual size in the renderer (though we need to know it for
initialization purposes obviously)

Next, the importance-sampled data; this takes the form of the following data:

 (1) the marginal CDF for all rows, containing a NORMALIZED function and cdf value for each row (both FP16 is fine I expect)
      -> or even just 16-bit fixed-point, since all values must be in [0, 1] so we get great precision
      -> good for the CDF but probably not great for the function? we may get inaccuracy problems

 (2) the conditional CDF for all rows, containing a NORMALIZED function and cdf value for each pixel

We should have all the info we need during sampling here!

http://www.pbr-book.org/3ed-2018/Light_Transport_I_Surface_Reflection/Sampling_Light_Sources.html#InfiniteAreaLights

So the main parts of the puzzle that we do not have yet are:

  * the binary search algorithm used (see PBR book), this should not be too difficult
  * the PDF calculations
  * the interpolation of sampled (u, v) somehow

For building the filtered data, for now we'll keep it to a 1-1 pixel mapping, but there is no
reason we can't use larger tiles in the future. If we assume the envmap data is a power of two
then this works nicely.

To do this, we would (I suppose) average the luminance over the entire tile? that should work
nicely I think.

So during rendering, we pick (u, v) uniform which are looked up in the CDF arrays. These map to
"tiles" with a given (u1, v1) -> (u2, v2) range, which we can then interpolate into.

just follow the PBR book... but I think the "func" is unnecessary here in Distribution1D as we
can just recompute it from the difference between the CDFs... (unless we lose accuracy here?)

Tomorrow:

 * implement a DataTexture abstraction in the renderer to upload 2D data with a specific size
 * update the environment map upload to just use that (with FP16x4 format), also load the FP16
   data + resolution info directly from a byte array, let's start doing this when possible?
 * filter the environment map data (luminance + sin(theta) adjustment) on the fly
 * implement the CDF generation in here for both the marginal and the conditional CDF
    (for now do a 1-1 mapping but don't make any assumptions in the renderer)
 * implement the renderer to do the importance-sampling
    (leave out the PDF calculation since we can't use it yet, but keep it commented out for later)


*/

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
