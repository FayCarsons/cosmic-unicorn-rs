use crate::ledmatrix;

use super::pixel::Pixel;

pub const FRAMEBUFFER_SIZE: usize = ledmatrix::WIDTH * ledmatrix::HEIGHT;

pub struct FrameBuffer([Pixel; FRAMEBUFFER_SIZE]);

impl FrameBuffer {
    pub const fn new() -> Self {
        Self([Pixel::splat(0); FRAMEBUFFER_SIZE])
    }

    pub fn get_ptr(&mut self) -> *mut Pixel {
        self.0.as_mut_ptr()
    }

    pub fn offset_from_2d_coord(x: usize, y: usize) -> usize {
        y * ledmatrix::WIDTH + x
    }
}
