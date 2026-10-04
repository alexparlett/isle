//! A surface's drawing target: a texture Smithay samples as the colour attachment of an FBO
//! Skia draws into, on the compositor's own context.

use freya_embed::{GlTarget, Gpu};
use smithay::{
    backend::renderer::{
        element::texture::TextureRenderBuffer,
        gles::{GlesRenderer, GlesTexture, ffi},
    },
    utils::{Physical, Size, Transform},
};

pub struct GlSurface {
    pub size: Size<i32, Physical>,
    pub target: GlTarget,
    pub buffer: TextureRenderBuffer<GlesTexture>,
    fbo: u32,
    stencil: u32,
}

impl GlSurface {
    pub fn new(
        renderer: &mut GlesRenderer,
        gpu: &mut Gpu,
        size: Size<i32, Physical>,
    ) -> Result<Self, String> {
        let (tex, fbo, stencil) = renderer
            .with_context(|gl| unsafe {
                let mut tex = 0;
                gl.GenTextures(1, &mut tex);
                gl.BindTexture(ffi::TEXTURE_2D, tex);
                gl.TexImage2D(
                    ffi::TEXTURE_2D,
                    0,
                    ffi::RGBA8 as i32,
                    size.w,
                    size.h,
                    0,
                    ffi::RGBA,
                    ffi::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
                for (param, value) in [
                    (ffi::TEXTURE_MIN_FILTER, ffi::LINEAR),
                    (ffi::TEXTURE_MAG_FILTER, ffi::LINEAR),
                    (ffi::TEXTURE_WRAP_S, ffi::CLAMP_TO_EDGE),
                    (ffi::TEXTURE_WRAP_T, ffi::CLAMP_TO_EDGE),
                ] {
                    gl.TexParameteri(ffi::TEXTURE_2D, param, value as i32);
                }
                gl.BindTexture(ffi::TEXTURE_2D, 0);

                let mut stencil = 0;
                gl.GenRenderbuffers(1, &mut stencil);
                gl.BindRenderbuffer(ffi::RENDERBUFFER, stencil);
                gl.RenderbufferStorage(ffi::RENDERBUFFER, ffi::STENCIL_INDEX8, size.w, size.h);
                gl.BindRenderbuffer(ffi::RENDERBUFFER, 0);

                let mut fbo = 0;
                gl.GenFramebuffers(1, &mut fbo);
                gl.BindFramebuffer(ffi::FRAMEBUFFER, fbo);
                gl.FramebufferTexture2D(
                    ffi::FRAMEBUFFER,
                    ffi::COLOR_ATTACHMENT0,
                    ffi::TEXTURE_2D,
                    tex,
                    0,
                );
                gl.FramebufferRenderbuffer(
                    ffi::FRAMEBUFFER,
                    ffi::STENCIL_ATTACHMENT,
                    ffi::RENDERBUFFER,
                    stencil,
                );
                let status = gl.CheckFramebufferStatus(ffi::FRAMEBUFFER);
                gl.BindFramebuffer(ffi::FRAMEBUFFER, 0);
                if status != ffi::FRAMEBUFFER_COMPLETE {
                    gl.DeleteFramebuffers(1, &fbo);
                    gl.DeleteRenderbuffers(1, &stencil);
                    gl.DeleteTextures(1, &tex);
                    return Err(format!("framebuffer incomplete: {status:#x}"));
                }
                Ok((tex, fbo, stencil))
            })
            .map_err(|e| e.to_string())??;

        let Some(target) = GlTarget::new(gpu, fbo, (size.w, size.h), 8) else {
            renderer
                .with_context(|gl| unsafe {
                    gl.DeleteFramebuffers(1, &fbo);
                    gl.DeleteRenderbuffers(1, &stencil);
                    gl.DeleteTextures(1, &tex);
                })
                .map_err(|e| e.to_string())?;
            return Err("Skia could not wrap the framebuffer".into());
        };
        // Safety: `tex` is a complete RGBA8 texture made above; the renderer owns it from here.
        let texture =
            unsafe { GlesTexture::from_raw(renderer, Some(ffi::RGBA8), false, tex, (size.w, size.h).into()) };
        let buffer = TextureRenderBuffer::from_texture(renderer, texture, 1, Transform::Normal, None);
        Ok(Self {
            size,
            target,
            buffer,
            fbo,
            stencil,
        })
    }

    /// Frees the framebuffer and stencil; the texture goes with the last element that holds it.
    pub fn release(self, renderer: &mut GlesRenderer) {
        let (fbo, stencil) = (self.fbo, self.stencil);
        drop(self);
        let _ = renderer.with_context(|gl| unsafe {
            gl.DeleteFramebuffers(1, &fbo);
            gl.DeleteRenderbuffers(1, &stencil);
        });
    }
}

/// Puts back the GL state Smithay's renderer expects after Skia has drawn.
///
/// # Safety
///
/// The compositor's context must be current.
pub unsafe fn restore(gl: &ffi::Gles2) {
    unsafe {
        let mut max_attribs = 0;
        gl.GetIntegerv(ffi::MAX_VERTEX_ATTRIBS, &mut max_attribs);
        gl.BindFramebuffer(ffi::FRAMEBUFFER, 0);
        gl.BindRenderbuffer(ffi::RENDERBUFFER, 0);
        gl.BindVertexArray(0);
        gl.BindBuffer(ffi::ARRAY_BUFFER, 0);
        gl.BindBuffer(ffi::ELEMENT_ARRAY_BUFFER, 0);
        gl.BindBuffer(ffi::PIXEL_PACK_BUFFER, 0);
        gl.BindBuffer(ffi::PIXEL_UNPACK_BUFFER, 0);
        for i in 0..max_attribs as u32 {
            gl.DisableVertexAttribArray(i);
        }
        gl.UseProgram(0);
        gl.ActiveTexture(ffi::TEXTURE0);
        gl.BindTexture(ffi::TEXTURE_2D, 0);
        gl.BindSampler(0, 0);
        gl.Disable(ffi::STENCIL_TEST);
        gl.Disable(ffi::DEPTH_TEST);
        gl.Disable(ffi::CULL_FACE);
        gl.Disable(ffi::SCISSOR_TEST);
        gl.Enable(ffi::BLEND);
        gl.BlendEquation(ffi::FUNC_ADD);
        gl.BlendFunc(ffi::ONE, ffi::ONE_MINUS_SRC_ALPHA);
        gl.ColorMask(ffi::TRUE, ffi::TRUE, ffi::TRUE, ffi::TRUE);
        gl.PixelStorei(ffi::PACK_ALIGNMENT, 4);
        gl.PixelStorei(ffi::UNPACK_ALIGNMENT, 4);
        gl.PixelStorei(ffi::PACK_ROW_LENGTH, 0);
        gl.PixelStorei(ffi::UNPACK_ROW_LENGTH, 0);
    }
}
