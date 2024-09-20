use crate::{
    constants::{HEIGHT, WIDTH},
    pixel::{Pixel, RGB},
};

pub type RawBuffer = [u32; 32 * 32];

pub trait FrameBuffer {
    fn as_bytes(&self) -> RawBuffer;
}

impl FrameBuffer for [Pixel; WIDTH * HEIGHT] {
    fn as_bytes(&self) -> RawBuffer {
        unsafe { core::mem::transmute(*self) }
    }
}
