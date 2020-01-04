#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{AsAttachment, AsBindTarget, BindTarget};
use js_sys::{Error, Float32Array, Object, Uint16Array, Uint8Array};
use serde::Serialize;
use std::marker::PhantomData;
use web_sys::{WebGl2RenderingContext as Context, WebGlTexture, WebglCompressedTextureS3tcSrgb};

#[derive(Debug, Serialize)]
pub enum TextureCompression {
    S3TC,
    ASTC,
    None,
}

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
impl RenderTarget for NotRenderable {}

pub fn supported_texture_compression(gl: &Context) -> TextureCompression {
    if let Ok(Some(_)) = gl.get_extension("WEBGL_compressed_texture_s3tc_srgb") {
        return TextureCompression::S3TC;
    }

    if let Ok(Some(_)) = gl.get_extension("WEBGL_compressed_texture_astc") {
        return TextureCompression::ASTC;
    }

    TextureCompression::None
}

#[derive(Debug)]
pub struct Texture<T> {
    gl: Context,

    handle: Option<WebGlTexture>,
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

    pub fn layers(&self) -> usize {
        self.layout.2
    }

    pub fn invalidate(&mut self) {
        self.layout = (0, 0, 0);
        self.handle = None;
    }

    pub fn reset(&mut self) {
        if let Some(texture_handle) = &self.handle {
            self.gl.delete_texture(Some(texture_handle));
        }

        self.invalidate();
    }

    pub fn is_invalid(&self) -> bool {
        self.handle.is_none()
    }

    fn create_texture(&mut self, cols: usize, rows: usize, layers: usize) -> bool {
        assert!(cols > 0 && rows > 0, "invalid texture layout requested");

        if self.layout != (cols, rows, layers) || self.handle.is_none() {
            if let Some(texture_handle) = &self.handle {
                self.gl.delete_texture(Some(texture_handle));
            }

            self.handle = self.gl.create_texture();
            self.layout = (cols, rows, layers);

            false
        } else {
            true
        }
    }
}

impl<T: TextureFormat> Texture<T> {
    fn filter_mode_for_format(&self) -> i32 {
        if T::Filterable::VALUE {
            Context::LINEAR as i32
        } else {
            Context::NEAREST as i32
        }
    }

    fn set_texture_parameters(&mut self, target: u32) {
        self.gl.tex_parameteri(
            target,
            Context::TEXTURE_MAG_FILTER,
            self.filter_mode_for_format(),
        );

        self.gl.tex_parameteri(
            target,
            Context::TEXTURE_MIN_FILTER,
            self.filter_mode_for_format(),
        );

        self.gl
            .tex_parameteri(target, Context::TEXTURE_WRAP_S, Context::REPEAT as i32);

        self.gl
            .tex_parameteri(target, Context::TEXTURE_WRAP_T, Context::REPEAT as i32);
    }
}

impl<T: TextureFormat<Compressed = True>> Texture<T> {
    pub fn create_compressed(
        &mut self,
        _rows: usize,
        _cols: usize,
        _data: usize,
    ) -> Result<(), Error> {
        unreachable!("compressed textures not implemented yet")
        // this would be the same as arrays but with TEXTURE_2D
    }

    pub fn create_array_compressed(
        &mut self,
        rows: usize,
        cols: usize,
        layers: usize,
    ) -> Result<(), Error> {
        assert_ne!(layers, 0, "texture array must have at least one layer");

        if self.create_texture(cols, rows, layers) {
            return Ok(()); // texture already created
        }

        if let Some(format) = T::COMPRESSION_FORMAT {
            self.check_compression_extension(format)?;
        } else {
            unreachable!("compressed texture format must declare COMPRESSION_FORMAT")
        }

        self.gl
            .bind_texture(Context::TEXTURE_2D_ARRAY, self.handle.as_ref());

        self.gl.tex_storage_3d(
            Context::TEXTURE_2D_ARRAY,
            1,
            T::GL_INTERNAL_FORMAT,
            cols as i32,
            rows as i32,
            layers as i32,
        );

        self.set_texture_parameters(Context::TEXTURE_2D_ARRAY);

        Ok(())
    }

    pub fn upload_compressed(
        &mut self,
        _rows: usize,
        _cols: usize,
        _data: &[T::Data],
    ) -> Result<(), Error> {
        unreachable!("compressed textures not implemented yet")
        // this would be the same as arrays but with TEXTURE_2D
    }

    pub fn upload_layer_compressed(
        &mut self,
        rows: usize,
        cols: usize,
        layer: usize,
        data: &[T::Data],
    ) {
        assert!((rows, cols) == (self.rows(), self.cols()));
        assert!(layer < self.layers());

        self.gl
            .bind_texture(Context::TEXTURE_2D_ARRAY, self.handle.as_ref());

        self.gl.compressed_tex_sub_image_3d_with_array_buffer_view(
            Context::TEXTURE_2D_ARRAY,
            0,
            0,
            0,
            layer as i32,
            self.cols() as i32,
            self.rows() as i32,
            1,
            T::GL_FORMAT,
            &T::into_texture_source_data(cols, rows, data),
        );

        // I've seen this occur on Chrome, although the call still goes through. It
        // doesn't happen on Firefox which suggests some browser inconsistency, but
        // pretty sure this is allowed by the spec.

        if self.gl.get_error() == Context::INVALID_ENUM {
            log::warn!("spurious Chrome WebGL warning?");
        }
    }

    fn check_compression_extension(&mut self, format: TextureCompression) -> Result<(), Error> {
        match format {
            TextureCompression::S3TC => {
                if let Err(_) | Ok(None) =
                    self.gl.get_extension("WEBGL_compressed_texture_s3tc_srgb")
                {
                    return Err(Error::new("S3TC compression requested but not supported"));
                }
            }
            TextureCompression::ASTC => {
                if let Err(_) | Ok(None) = self.gl.get_extension("WEBGL_compressed_texture_astc") {
                    return Err(Error::new("ASTC compression requested but not supported"));
                }
            }
            TextureCompression::None => {}
        }

        Ok(())
    }
}

impl<T: TextureFormat<Compressed = False>> Texture<T> {
    pub fn create(&mut self, cols: usize, rows: usize) {
        if self.create_texture(cols, rows, 0) {
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

        self.set_texture_parameters(Context::TEXTURE_2D);
    }

    pub fn upload(&mut self, cols: usize, rows: usize, data: &[T::Data]) {
        self.create(cols, rows);

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
                Some(&T::into_texture_source_data(cols, rows, data)),
            )
            .unwrap();
    }

    pub fn create_array(&mut self, cols: usize, rows: usize, layers: usize) {
        assert_ne!(layers, 0, "texture array must have at least one layer");

        if self.create_texture(cols, rows, layers) {
            return; // texture already created
        }

        self.gl
            .bind_texture(Context::TEXTURE_2D_ARRAY, self.handle.as_ref());

        self.gl.tex_storage_3d(
            Context::TEXTURE_2D_ARRAY,
            1,
            T::GL_INTERNAL_FORMAT,
            cols as i32,
            rows as i32,
            layers as i32,
        );

        self.set_texture_parameters(Context::TEXTURE_2D_ARRAY);
    }

    pub fn upload_layer(&mut self, cols: usize, rows: usize, layer: usize, data: &[T::Data]) {
        assert!((cols, rows) == (self.cols(), self.rows()));
        assert!(layer < self.layers());

        self.gl
            .bind_texture(Context::TEXTURE_2D_ARRAY, self.handle.as_ref());

        self.gl
            .tex_sub_image_3d_with_opt_array_buffer_view(
                Context::TEXTURE_2D_ARRAY,
                0,
                0,
                0,
                layer as i32,
                cols as i32,
                rows as i32,
                1,
                T::GL_FORMAT,
                T::GL_TYPE,
                Some(&T::into_texture_source_data(cols, rows, data)),
            )
            .unwrap();
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
        BindTarget::Texture(self.handle.as_ref(), self.layers() != 0)
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

    const COMPRESSION_FORMAT: Option<TextureCompression>;
    const GL_INTERNAL_FORMAT: u32;
    const GL_FORMAT: u32;
    const GL_TYPE: u32;

    fn into_texture_source_data(_cols: usize, _rows: usize, _layer: &[Self::Data]) -> Object {
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
#[derive(Debug)]
pub struct SRGBA8;
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct SRGB_S3TC_DXT1;

impl TextureFormat for RGBA32UI {
    type Data = u32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::RGBA32UI;
    const GL_FORMAT: u32 = Context::RGBA_INTEGER;
    const GL_TYPE: u32 = Context::UNSIGNED_INT;
}

impl TextureFormat for RGBA32F {
    type Data = f32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::RGBA32F;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::FLOAT;
}

impl TextureFormat for R32F {
    type Data = f32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::R32F;
    const GL_FORMAT: u32 = Context::RED;
    const GL_TYPE: u32 = Context::FLOAT;
}

impl TextureFormat for R32UI {
    type Data = u32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::R32UI;
    const GL_FORMAT: u32 = Context::RED;
    const GL_TYPE: u32 = Context::UNSIGNED_INT;
}

impl TextureFormat for RG32F {
    type Data = f32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::RG32F;
    const GL_FORMAT: u32 = Context::RG;
    const GL_TYPE: u32 = Context::FLOAT;

    fn into_texture_source_data(cols: usize, rows: usize, layer: &[Self::Data]) -> Object {
        assert!(layer.len() == cols * rows * 2);

        Float32Array::from(layer).into()
    }
}

impl TextureFormat for RGBA8 {
    type Data = u8;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::RGBA8;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::UNSIGNED_BYTE;

    fn into_texture_source_data(cols: usize, rows: usize, layer: &[Self::Data]) -> Object {
        assert!(layer.len() == cols * rows * 4);

        Uint8Array::from(layer).into()
    }
}

impl TextureFormat for R8 {
    type Data = u8;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::R8;
    const GL_FORMAT: u32 = Context::RED;
    const GL_TYPE: u32 = Context::UNSIGNED_BYTE;

    fn into_texture_source_data(cols: usize, rows: usize, layer: &[Self::Data]) -> Object {
        assert!(layer.len() == cols * rows);

        Uint8Array::from(layer).into()
    }
}

impl TextureFormat for R16F {
    type Data = u16;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::R16F;
    const GL_FORMAT: u32 = Context::RED;
    const GL_TYPE: u32 = Context::HALF_FLOAT;

    fn into_texture_source_data(cols: usize, rows: usize, layer: &[Self::Data]) -> Object {
        assert!(layer.len() == cols * rows);

        Uint16Array::from(layer).into()
    }
}

impl TextureFormat for RGBA16F {
    type Data = u16;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::RGBA16F;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::HALF_FLOAT;

    fn into_texture_source_data(cols: usize, rows: usize, layer: &[Self::Data]) -> Object {
        assert!(layer.len() == cols * rows * 4);

        Uint16Array::from(layer).into()
    }
}

impl TextureFormat for D24S8 {
    type Data = u32;

    type Compressed = False;
    type Filterable = False;
    type Renderable = DepthStencil;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::DEPTH24_STENCIL8;
    const GL_FORMAT: u32 = Context::DEPTH_STENCIL;
    const GL_TYPE: u32 = Context::UNSIGNED_INT_24_8;
}

impl TextureFormat for RGB10A2 {
    type Data = u32;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::RGB10_A2;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::UNSIGNED_INT_2_10_10_10_REV;
}

impl TextureFormat for SRGBA8 {
    type Data = u8;

    type Compressed = False;
    type Filterable = True;
    type Renderable = Color;

    const COMPRESSION_FORMAT: Option<TextureCompression> = None;
    const GL_INTERNAL_FORMAT: u32 = Context::SRGB8_ALPHA8;
    const GL_FORMAT: u32 = Context::RGBA;
    const GL_TYPE: u32 = Context::UNSIGNED_BYTE;

    fn into_texture_source_data(cols: usize, rows: usize, layer: &[Self::Data]) -> Object {
        assert!(layer.len() == cols * rows * 4);

        Uint8Array::from(layer).into()
    }
}

impl TextureFormat for SRGB_S3TC_DXT1 {
    type Data = u8;

    type Compressed = True;
    type Filterable = True;
    type Renderable = NotRenderable;

    const COMPRESSION_FORMAT: Option<TextureCompression> = Some(TextureCompression::S3TC);
    const GL_INTERNAL_FORMAT: u32 = WebglCompressedTextureS3tcSrgb::COMPRESSED_SRGB_S3TC_DXT1_EXT;
    const GL_FORMAT: u32 = WebglCompressedTextureS3tcSrgb::COMPRESSED_SRGB_S3TC_DXT1_EXT;
    const GL_TYPE: u32 = 0;

    fn into_texture_source_data(cols: usize, rows: usize, layer: &[Self::Data]) -> Object {
        assert!(layer.len() == cols * rows / 2);

        Uint8Array::from(layer).into()
    }
}
