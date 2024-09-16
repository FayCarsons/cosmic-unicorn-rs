use core::usize;

use cortex_m::delay::Delay;
use cortex_m::singleton;
use cortex_m_rt::interrupt;
use embedded_hal::digital::OutputPin;
use pio::ProgramWithDefines;

use hal::dma::{double_buffer, DMAExt, EndlessWriteTarget};
use hal::pio::{Buffers, PinDir, Tx};
use rp_pico::hal;
use rp_pico::pac::PIO0;

use hal::dma::{
    double_buffer::{ReadNext, Transfer},
    Channel, CH0, CH1,
};
use hal::gpio::{PinState, Pins};
use hal::pio::{PIOBuilder, PIOExt, SM0};
use hal::sio::Sio;

use crate::framebuffer::FrameBuffer;
use crate::pixel;

pub const GAMMA_LUT_14BIT: [u16; 256] = [
    0, 0, 0, 1, 2, 3, 4, 6, 8, 10, 13, 16, 20, 23, 28, 32, 37, 42, 48, 54, 61, 67, 75, 82, 90, 99,
    108, 117, 127, 137, 148, 159, 170, 182, 195, 207, 221, 234, 249, 263, 278, 294, 310, 326, 343,
    361, 379, 397, 416, 435, 455, 475, 496, 517, 539, 561, 583, 607, 630, 654, 679, 704, 730, 756,
    783, 810, 838, 866, 894, 924, 953, 983, 1014, 1045, 1077, 1110, 1142, 1176, 1210, 1244, 1279,
    1314, 1350, 1387, 1424, 1461, 1499, 1538, 1577, 1617, 1657, 1698, 1739, 1781, 1823, 1866, 1910,
    1954, 1998, 2044, 2089, 2136, 2182, 2230, 2278, 2326, 2375, 2425, 2475, 2525, 2577, 2629, 2681,
    2734, 2787, 2841, 2896, 2951, 3007, 3063, 3120, 3178, 3236, 3295, 3354, 3414, 3474, 3535, 3596,
    3658, 3721, 3784, 3848, 3913, 3978, 4043, 4110, 4176, 4244, 4312, 4380, 4449, 4519, 4589, 4660,
    4732, 4804, 4876, 4950, 5024, 5098, 5173, 5249, 5325, 5402, 5479, 5557, 5636, 5715, 5795, 5876,
    5957, 6039, 6121, 6204, 6287, 6372, 6456, 6542, 6628, 6714, 6801, 6889, 6978, 7067, 7156, 7247,
    7337, 7429, 7521, 7614, 7707, 7801, 7896, 7991, 8087, 8183, 8281, 8378, 8477, 8576, 8675, 8775,
    8876, 8978, 9080, 9183, 9286, 9390, 9495, 9600, 9706, 9812, 9920, 10027, 10136, 10245, 10355,
    10465, 10576, 10688, 10800, 10913, 11027, 11141, 11256, 11371, 11487, 11604, 11721, 11840,
    11958, 12078, 12198, 12318, 12440, 12562, 12684, 12807, 12931, 13056, 13181, 13307, 13433,
    13561, 13688, 13817, 13946, 14076, 14206, 14337, 14469, 14602, 14735, 14868, 15003, 15138,
    15273, 15410, 15547, 15685, 15823, 15962, 16102, 16242, 16383,
];

pub const WIDTH: usize = 32;
pub const HEIGHT: usize = 32;
const ROW_COUNT: usize = 16;
const BCD_FRAME_COUNT: usize = 14;
const BCD_FRAME_BYTES: usize = 72;
const ROW_BYTES: usize = BCD_FRAME_COUNT * BCD_FRAME_BYTES;

// We divide this by 4 because in this port our buffer is u32 vs the original's u8

const BYTE_ALIGNED_BITSTREAM_LENGTH: usize = ROW_COUNT * ROW_BYTES;
const BITSTREAM_LENGTH: usize = BYTE_ALIGNED_BITSTREAM_LENGTH / 4;

type BitStream = [u32; BITSTREAM_LENGTH];
type ByteAlignedBitStream = [u8; BITSTREAM_LENGTH * 4];

type BitstreamTransfer = Transfer<
    Channel<CH0>,
    Channel<CH1>,
    &'static mut [u32; BITSTREAM_LENGTH],
    Tx<(PIO0, SM0)>,
    ReadNext<&'static mut [u32; BITSTREAM_LENGTH]>,
>;

pub struct CosmicUnicorn {
    transfer: BitstreamTransfer,
    bitstream: *mut u8,
    tx_buf: *mut u8,
}

// Pins used for PIO+DMA
//  - 13: column clock (sideset), init 0
//  - 14: column data  (out base), init 0
//  - 15: column latch, init 0
//  - 16: column blank, init 1
//  All init 1 >>
//  - 17: row select bit 0
//  - 18: row select bit 1
//  - 19: row select bit 2
//  - 20: row select bit 3

static mut BITSTREAM: BitStream = [0; BITSTREAM_LENGTH];
static mut TX_BUF: BitStream = [0; BITSTREAM_LENGTH];

const ROW_SELECT_OFFSET: usize = 1;
const BCD_TICKS_OFFSET: usize = 68;

impl CosmicUnicorn {
    unsafe fn init_bitstream(bitstream: &mut BitStream) {
        let byte_aligned = bitstream.as_mut_ptr().cast::<u8>();

        for row in 0..16 {
            for frame in 0..BCD_FRAME_COUNT {
                let offset = row * ROW_BYTES + frame * BCD_FRAME_BYTES;

                let bitstream_ptr = byte_aligned.add(offset);
                bitstream_ptr.write(63);

                bitstream_ptr.add(ROW_SELECT_OFFSET).write(row as u8);

                let bcd_ticks = 1 << frame;
                bitstream_ptr
                    .add(BCD_TICKS_OFFSET)
                    .write((bcd_ticks & 0xff) >> 0);
                bitstream_ptr
                    .add(BCD_TICKS_OFFSET + 1)
                    .write((bcd_ticks & 0xff00) >> 8);
                bitstream_ptr
                    .add(BCD_TICKS_OFFSET + 2)
                    .write((bcd_ticks & 0xff0000) >> 16);
                bitstream_ptr
                    .add(BCD_TICKS_OFFSET + 3)
                    .write((bcd_ticks & 0xff000000) >> 24);
            }
        }
    }

    pub fn clear(&mut self) {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                self.set_pixel(x, y, pixel::Pixel::splat(0))
            }
        }
    }

    fn init_pins(pins: Pins, delay: &mut Delay) {
        let mut clock = pins.gpio13.into_push_pull_output_in_state(PinState::Low);
        let mut data = pins.gpio14.into_push_pull_output_in_state(PinState::Low);
        let mut latch = pins.gpio15.into_push_pull_output_in_state(PinState::Low);
        let mut blank = pins.gpio16.into_push_pull_output_in_state(PinState::High);

        let _ = pins.gpio17.into_push_pull_output_in_state(PinState::High);
        let _ = pins.gpio18.into_push_pull_output_in_state(PinState::High);
        let _ = pins.gpio19.into_push_pull_output_in_state(PinState::High);
        let _ = pins.gpio20.into_push_pull_output_in_state(PinState::High);

        delay.delay_ms(100);

        let reg1 = 0b1111111111001110;

        for _ in 0..11 {
            for i in 0..16 {
                if reg1 & (1 << (15 - i)) != 0 {
                    data.set_high();
                } else {
                    data.set_low();
                }

                delay.delay_us(10);
                clock.set_high();
                delay.delay_us(10);
                clock.set_low();
            }
        }

        for i in 0..16 {
            if reg1 & (1 << (15 - i)) != 0 {
                data.set_high();
            } else {
                data.set_low();
            }

            delay.delay_us(10);
            clock.set_high();
            delay.delay_us(10);
            clock.set_low();

            if i == 4 {
                latch.set_high();
            }
        }

        latch.set_low();

        // reapply the blank as the above seems to cause a slight glow
        blank.set_low();
        delay.delay_us(10);
        blank.set_high();
    }

    pub fn new(mut pac: rp_pico::pac::Peripherals, delay: &mut Delay) -> Self {
        let sio = Sio::new(pac.SIO);

        let pins = hal::gpio::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        Self::init_pins(pins, delay);

        let program = pio_proc::pio_file!("cosmic_unicorn.pio");
        let (mut pio, sm0, a, b, c) = pac.PIO0.split(&mut pac.RESETS);
        let installed = pio.install(&program.program).unwrap();

        let (mut sm, _, tx) = PIOBuilder::from_installed_program(installed)
            .out_pins(17, 4) // ROW_BIT_0 (17) to ROW_BIT_3 (20)
            .set_pins(14, 3) // COLUMN_DATA (14), COLUMN_LATCH (15), COLUMN_BLANK (16)
            .side_set_pin_base(13) // COLUMN_CLOCK (13)
            .autopull(true)
            .pull_threshold(32)
            .buffers(Buffers::OnlyTx)
            .build(sm0);

        sm.set_pindirs((13..=20).zip(core::iter::repeat(PinDir::Output)));

        unsafe {
            Self::init_bitstream(&mut BITSTREAM);
        }

        let dma = pac.DMA.split(&mut pac.RESETS);

        let transfer = unsafe {
            double_buffer::Config::new((dma.ch0, dma.ch1), &mut BITSTREAM, tx)
                .start()
                .read_next(&mut TX_BUF)
        };

        let bitstream = unsafe { BITSTREAM.as_mut_ptr().cast::<u8>() };
        let tx_buf = unsafe { TX_BUF.as_mut_ptr().cast::<u8>() };

        sm.start();

        Self {
            transfer,
            bitstream,
            tx_buf,
        }
    }

    pub fn set_pixel<P>(&mut self, x: usize, y: usize, pixel: P)
    where
        P: super::pixel::RGB,
    {
        let [r, g, b] = pixel.to_rgb();
        let (r, g, b) = (r >> 8, g >> 8, b >> 8);

        let mut x = (WIDTH - 1) - x;
        let mut y = (HEIGHT - 1) - y;

        if y < 16 {
            x += 32;
        } else {
            y -= 16;
        }

        use super::gamma::GAMMA_LUT_14BIT as GAMMA;
        let (mut r, mut g, mut b) = (GAMMA[r as usize], GAMMA[g as usize], GAMMA[b as usize]);

        // for each row:
        //   for each bcd frame:
        //            0: 00111111                           // row pixel count (minus
        //            one)
        //      1  - 64: xxxxxbgr, xxxxxbgr, xxxxxbgr, ...  // pixel data
        //      65 - 67: xxxxxxxx, xxxxxxxx, xxxxxxxx       // dummy bytes to dword
        //      align
        //           68: xxxxrrrr                           // row select bits
        //      69 - 71: tttttttt, tttttttt, tttttttt       // bcd tick count
        //      (0-65536)
        //
        //  .. and back to the start

        let base_offset = y * ROW_BYTES + 2 + x;

        // set the appropriate bits in the separate bcd frames
        for frame in 0..BCD_FRAME_COUNT {
            let offset = base_offset + (BCD_FRAME_BYTES * frame);

            let red_bit = r & 0b1;
            let green_bit = g & 0b1;
            let blue_bit = b & 0b1;

            let pixel = ((blue_bit << 0) | (green_bit << 1) | (red_bit << 2)) as u8;
            unsafe {
                self.bitstream
                    .add(offset)
                    .cast::<u8>()
                    .write_volatile(pixel);
            }

            r >>= 1;
            g >>= 1;
            b >>= 1;
        }
    }

    pub fn update<B>(&mut self, buffer: B)
    where
        B: FrameBuffer,
    {
        let rawbuf = buffer.as_bytes().as_mut_ptr();
        let mut offset = 0;

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let col = unsafe { rawbuf.add(offset).read() };

                let r = (col & 0xff0000) >> 16;
                let g = (col & 0x00ff00) >> 8;
                let b = (col & 0x0000ff) >> 0;

                self.set_pixel(x, y, super::pixel::Pixel::new(r as u8, g as u8, b as u8));
                offset += 1;
            }
        }
    }
}
