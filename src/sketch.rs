use crate::{
    framebuffer::{FrameBuffer, RawBuffer},
    pixel::Pixel,
};
use core::ops::Rem;

use super::constants::*;

pub struct Sketch {
    buffer: [Pixel; WIDTH * HEIGHT],
    timer: u32,
    color_index: u8,
}

const INTERVAL: u32 = 30;

const RED: Pixel = Pixel::new(255, 1, 1);
const GREEN: Pixel = Pixel::new(1, 255, 1);
const BLUE: Pixel = Pixel::new(1, 1, 255);
const COLORS: [Pixel; 3] = [RED, GREEN, BLUE];

impl Sketch {
    pub fn new() -> Self {
        Self {
            buffer: [Pixel::BLACK; WIDTH * HEIGHT],
            timer: 0,
            color_index: 0,
        }
    }

    pub fn update(&mut self) {
        if self.timer.rem(INTERVAL) == 0 {
            self.buffer.fill(COLORS[self.color_index as usize]);
            self.color_index = (self.color_index + 1) % 3;
        }

        self.timer = self.timer.wrapping_add(1)
    }
}
impl FrameBuffer for Sketch {
    fn as_bytes(&self) -> RawBuffer {
        unsafe { core::mem::transmute(self.buffer) }
    }
}
