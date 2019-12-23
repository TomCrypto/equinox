#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{AsAttachment, AsBindTarget, BindTarget};
use js_sys::{Float32Array, Object, Uint16Array, Uint8Array};
use std::marker::PhantomData;
use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer, WebGlTexture};

pub trait Boolean {
    const VALUE: bool;
}

pub struct True;
pub struct False;

impl Boolean for True {
    const VALUE: bool = true;
}
impl Boolean for False {
    const VALUE: bool = false;
}

pub trait RenderTarget {}

pub struct Color;
pub struct DepthStencil;
pub struct NotRenderable;

impl RenderTarget for Color {}
impl RenderTarget for DepthStencil {}

pub trait AsPixelSource {
    fn as_pixel_source(&self) -> Option<&WebGlBuffer>;
}

#[derive(Debug)]
pub struct Texture<T> {
    gl: Context,

    pub handle: Option<WebGlTexture>,
    layout: (usize, usize, usize),
    format: PhantomData<T>,
}

impl<T> Texture<T> {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            layout: (0, 0, 0),
            format: PhantomData,
        }
    }

    pub fn cols(&self) -> usize {
        self.layout.0
    }

    pub fn rows(&self) -> usize {
        self.layout.1
    }

    pub fn invalidate(&mut self) {
        self.handle = None;
    }

    fn create_texture(&mut self, cols: usize, rows: usize, mip_levels: usize) -> bool {
        assert!(
            cols > 0 && rows > 0 && mip_levels > 0,
            "invalid texture layout/size requested"
        );

        if self.layout != (cols, rows, mip_levels) || self.handle.is_none() {
            if let Some(texture_handle) = &self.handle {
                self.gl.delete_texture(Some(texture_handle));
            }

            self.handle = self.gl.create_texture();
            self.layout = (cols, rows, mip_levels);

            if let Err(_) | Ok(None) = self.gl.get_extension("OES_texture_float_linear") {
                panic!("the WebGL2 extension `OES_texture_float_linear' is unavailable");
            }

            false
        } else {
            true
        }
    }
}

impl<T: TextureFormat> Texture<T> {
    fn mag_filter_for_format(&self) -> i32 {
        if T::Filterable::VALUE {
            Context::LINEAR as i32
        } else {
            Context::NEAREST as i32
        }
    }

    fn min_filter_for_format(&self, mipped: bool) -> i32 {
        if !T::Filterable::VALUE && mipped {
            unreachable!("mipped texture with non-filterable format requested");
        }

        if T::Filterable::VALUE && mipped {
            Context::LINEAR_MIPMAP_LINEAR as i32
        } else if T::Filterable::VALUE {
            Context::LINEAR as i32
        } else {
            Context::NEAREST as i32
        }
    }

    fn set_texture_parameters(&mut self, mipped: bool) {
        self.gl.tex_parameteri(
            Context::TEXTURE_2D,
            Context::TEXTURE_MAG_FILTER,
            self.mag_filter_for_format(),
        );

        self.gl.tex_parameteri(
            Context::TEXTURE_2D,
            Context::TEXTURE_MIN_FILTER,
            self.min_filter_for_format(mipped),
        );

        self.gl.tex_parameteri(
            Context::TEXTURE_2D,
            Context::TEXTURE_WRAP_S,
            Context::REPEAT as i32,
        );

        self.gl.tex_parameteri(
            Context::TEXTURE_2D,
            Context::TEXTURE_WRAP_T,
            Context::REPEAT as i32,
        );
    }
}

impl<T: TextureFormat<Filterable = True, Compressed = True>> Texture<T> {
    // Compressed textures can never be rendered to by hardware, so they have to be
    // initialized with data; it doesn't make sense to create an uninitialized one.

    pub fn upload_compressed(&mut self, _rows: usize, _cols: usize, _data: &[T::Data]) {
        unimplemented!("compressed textures are not implemented yet")
    }
}

impl<T: TextureFormat<Filterable = True, Compressed = False>> Texture<T> {
    pub fn create_mipped(&mut self, cols: usize, rows: usize) {
        let mip_levels = Self::mip_levels(cols, rows);

        if self.create_texture(cols, rows, mip_levels) {
            return; // mipped texture already created
        }

        self.gl
            .bind_texture(Context::TEXTURE_2D, self.handle.as_ref());

        self.gl.tex_storage_2d(
            Context::TEXTURE_2D,
            mip_levels as i32,
            T::GL_INTERNAL_FORMAT,
            cols as i32,
            rows as i32,
        );

        self.set_texture_parameters(true);
    }

    pub fn upload_mipped(&mut self, cols: usize, rows: usize, data: &[T::Data]) {
        let mip_levels = Self::mip_levels(cols, rows);

        self.create_mipped(cols, rows);

        let level_slices = T::parse(cols, rows, mip_levels, data);

        assert_eq!(level_slices.len(), mip_levels);

        self.gl.bind_buffer(Context::PIXEL_UNPACK_BUFFER, None);
        self.gl
            .bind_texture(Context::TEXTURE_2D, self.handle.as_ref());

        for (level, level_slice) in level_slices.into_iter().enumerate() {
            let level_cols = (cols / (1 << level)).max(1);
            let level_rows = (rows / (1 << level)).max(1);

            self.gl
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                    Context::TEXTURE_2D,
                    0,
                    0,
                    level as i32,
                    level_cols as i32,
                    level_rows as i32,
                    T::GL_FORMAT,
                    T::GL_TYPE,
                    Some(&level_slice),
                )
                .unwrap();
        }
    }

    pub fn gen_mipmaps(&mut self) {
        self.gl.hint(Context::GENERATE_MIPMAP_HINT, Context::NICEST);
        self.gl
            .bind_texture(Context::TEXTURE_2D, self.handle.as_ref());
        self.gl.generate_mipmap(Context::TEXTURE_2D);
    }

    pub fn level_dimensions(&self, level: usize) -> (usize, usize) {
        let level_cols = (self.cols() / (1 << level)).max(1);
        let level_rows = (self.rows() / (1 << level)).max(1);

        (level_cols, level_rows)
    }

    fn mip_levels(cols: usize, rows: usize) -> usize {
        1 + (cols.max(rows) as f32).log2().floor() as usize
    }
}

impl<T: TextureFormat<Compressed = False>> Texture<T> {
    pub fn create(&mut self, cols: usize, rows: usize) {
        if self.create_texture(cols, rows, 1) {
            return; // texture already created
        }

        self.gl
            .bind_texture(Context::TEXTURE_2D, self.handle.as_ref());

        self.gl.tex_storage_2d(
            Context::TEXTURE_2D,
            1,
            T::GL_INTERNAL_FORMAT,
            cols as i32,
            rows as i32,
        );

        self.set_texture_parameters(false);
    }

    pub fn upload(&mut self, cols: usize, rows: usize, data: &[T::Data]) {
        self.create(cols, rows);

        let level_slices = T::parse(cols, rows, 1, data);

        assert_eq!(level_slices.len(), 1);

        self.gl.bind_buffer(Context::PIXEL_UNPACK_BUFFER, None);
        self.gl
            .bind_texture(Context::TEXTURE_2D, self.handle.as_ref());

        self.gl
            .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                Context::TEXTURE_2D,
                0,
                0,
                0,
                cols as i32,
                rows as i32,
                T::GL_FORMAT,
                T::GL_TYPE,
                Some(&level_slices[0]),
            )
            .unwrap();
    }

    pub fn copy_from(&mut self, source: &dyn AsPixelSource) {
        self.gl
            .bind_texture(Context::TEXTURE_2D, self.handle.as_ref());

        self.gl
            .bind_buffer(Context::PIXEL_UNPACK_BUFFER, source.as_pixel_source());

        self.gl
            .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_i32(
                Context::TEXTURE_2D,
                0,
                0,
                0,
                self.cols() as i32,
                self.rows() as i32,
                T::GL_FORMAT,
                T::GL_TYPE,
                0,
            )
            .expect("texSubImage2D with a PBO source cannot fail");
    }
}

impl<T: TextureFormat> AsAttachment for Texture<T> {
    type Target = T::Renderable;

    fn as_attachment(&self) -> Option<&WebGlTexture> {
        self.handle.as_ref()
    }

    fn attachment_dimensions(&self) -> (usize, usize) {
        (self.cols(), self.rows())
    }
}

impl<T: TextureFormat> AsBindTarget for Texture<T> {
    fn bind_target(&self) -> BindTarget {
        BindTarget::Texture(self.handle.as_ref())
    }
}

impl<T> Drop for Texture<T> {
    fn drop(&mut self) {
        if let Some(texture_handle) = &self.handle {
            self.gl.delete_texture(Some(texture_handle));
        }
    }
}

pub trait TextureFormat {
    type Data;

    type Filterable: Boolean;
    type Compressed: Boolean;
    type Renderable: RenderTarget;

    const GL_INTERNAL_FORMAT: u32;
    const GL_FORMAT: u32;
    const GL_TYPE: u32;

    fn parse(_cols: usize, _rows: usize, _levels: usize, _data: &[Self::Data]) -> Vec<Object> {
        unimplemented!("texture data upload is not yet implemented for this texture format")
    }
}

#[derive(Debug)]
pub struct RGBA32UI;
#[derive(Debug)]
pub struct RGBA32F;
#[derive(Debug)]
pub struct R32F;
#[derive(Debug)]
pub struct R32UI;
#[derive(Debug)]
pub struct RG32F;
#[derive(Debug)]
pub struct RGBA8;
#[derive(Debug)]
pub struct R8;
#[derive(Debug)]
pub struct R16F;
#[derive(Debug)]
pub struct RGBA16F;
#[derive(Debug)]
pub struct D24S8;
#[derive(Debug)]
pub struct RGB10A2;

impl TextureFormat for RGBA32UI {
    type Data = u32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::RGBA32UI;
    const GL_FORMAT: u32 = Context::RGBA_INTEGER;
    const GL_TYPE: u32 = Context::UNSIGNED_INT;
}

impl TextureFormat for RGBA32F {
    type Data = f32;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::RGBA32F;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::FLOAT;
}

impl TextureFormat for R32F {
    type Data = f32;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::R32F;
    const GL_FORMAT: u32 = Context::RED;
    const GL_TYPE: u32 = Context::FLOAT;
}

impl TextureFormat for R32UI {
    type Data = u32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::R32UI;
    const GL_FORMAT: u32 = Context::RED;
    const GL_TYPE: u32 = Context::UNSIGNED_INT;
}

impl TextureFormat for RG32F {
    type Data = f32;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::RG32F;
    const GL_FORMAT: u32 = Context::RG;
    const GL_TYPE: u32 = Context::FLOAT;

    fn parse(cols: usize, rows: usize, levels: usize, mut data: &[Self::Data]) -> Vec<Object> {
        let mut views = Vec::with_capacity(levels);

        for level in 0..levels {
            let level_cols = (cols / (1 << level)).max(1);
            let level_rows = (rows / (1 << level)).max(1);
            let level_size = level_cols * level_rows * 2;

            assert!(data.len() >= level_size);

            let (level_data, remaining) = data.split_at(level_size);

            views.push(Float32Array::from(level_data).into());

            data = remaining;
        }

        assert!(data.is_empty());

        views
    }
}

impl TextureFormat for RGBA8 {
    type Data = u8;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::RGBA8;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::UNSIGNED_BYTE;

    fn parse(cols: usize, rows: usize, levels: usize, mut data: &[Self::Data]) -> Vec<Object> {
        let mut views = Vec::with_capacity(levels);

        for level in 0..levels {
            let level_cols = (cols / (1 << level)).max(1);
            let level_rows = (rows / (1 << level)).max(1);
            let level_size = level_cols * level_rows * 4;

            assert!(data.len() >= level_size);

            let (level_data, remaining) = data.split_at(level_size);

            views.push(Uint8Array::from(level_data).into());

            data = remaining;
        }

        assert!(data.is_empty());

        views
    }
}

impl TextureFormat for R8 {
    type Data = u8;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::R8;
    const GL_FORMAT: u32 = Context::RED;
    const GL_TYPE: u32 = Context::UNSIGNED_BYTE;

    fn parse(cols: usize, rows: usize, levels: usize, mut data: &[Self::Data]) -> Vec<Object> {
        let mut views = Vec::with_capacity(levels);

        for level in 0..levels {
            let level_cols = (cols / (1 << level)).max(1);
            let level_rows = (rows / (1 << level)).max(1);
            let level_size = level_cols * level_rows;

            assert!(data.len() >= level_size);

            let (level_data, remaining) = data.split_at(level_size);

            views.push(Uint8Array::from(level_data).into());

            data = remaining;
        }

        assert!(data.is_empty());

        views
    }
}

impl TextureFormat for R16F {
    type Data = u16;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::R16F;
    const GL_FORMAT: u32 = Context::RED;
    const GL_TYPE: u32 = Context::HALF_FLOAT;

    fn parse(cols: usize, rows: usize, levels: usize, mut data: &[Self::Data]) -> Vec<Object> {
        let mut views = Vec::with_capacity(levels);

        for level in 0..levels {
            let level_cols = (cols / (1 << level)).max(1);
            let level_rows = (rows / (1 << level)).max(1);
            let level_size = level_cols * level_rows;

            assert!(data.len() >= level_size);

            let (level_data, remaining) = data.split_at(level_size);

            views.push(Uint16Array::from(level_data).into());

            data = remaining;
        }

        assert!(data.is_empty());

        views
    }
}

impl TextureFormat for RGBA16F {
    type Data = u16;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::RGBA16F;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::HALF_FLOAT;

    fn parse(cols: usize, rows: usize, levels: usize, mut data: &[Self::Data]) -> Vec<Object> {
        let mut views = Vec::with_capacity(levels);

        for level in 0..levels {
            let level_cols = (cols / (1 << level)).max(1);
            let level_rows = (rows / (1 << level)).max(1);
            let level_size = level_cols * level_rows * 4;

            assert!(data.len() >= level_size);

            let (level_data, remaining) = data.split_at(level_size);

            views.push(Uint16Array::from(level_data).into());

            data = remaining;
        }

        assert!(data.is_empty());

        views
    }
}

impl TextureFormat for D24S8 {
    type Data = u32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = DepthStencil;

    const GL_INTERNAL_FORMAT: u32 = Context::DEPTH24_STENCIL8;
    const GL_FORMAT: u32 = Context::DEPTH_STENCIL;
    const GL_TYPE: u32 = Context::UNSIGNED_INT_24_8;
}

impl TextureFormat for RGB10A2 {
    type Data = u32;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const GL_INTERNAL_FORMAT: u32 = Context::RGB10_A2;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::UNSIGNED_INT_2_10_10_10_REV;
}
