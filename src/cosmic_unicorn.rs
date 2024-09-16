use core::ops::Rem;

use cortex_m::delay::Delay;
use embedded_hal::digital::OutputPin;

use hal::dma::double_buffer;
use hal::pio::{Buffers, PinDir, Tx};
use rp_pico::hal;
use rp_pico::pac::PIO0;

use hal::dma::{
    double_buffer::{ReadNext, Transfer},
    Channel, CH0, CH1,
};
use hal::gpio::{Error, PinState, Pins};
use hal::pio::{PIOBuilder, SM0};

use crate::builder::CosmicBuilder;

use super::constants::*;
use super::framebuffer::FrameBuffer;
use super::pixel;
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

                let bcd_ticks = 1u32 << frame;
                bitstream_ptr
                    .add(BCD_TICKS_OFFSET)
                    .write((bcd_ticks & 0xff).rem(256) as u8);
                bitstream_ptr
                    .add(BCD_TICKS_OFFSET + 1)
                    .write((bcd_ticks & 0xff00).wrapping_shr(8).rem(256) as u8);
                bitstream_ptr
                    .add(BCD_TICKS_OFFSET + 2)
                    .write((bcd_ticks & 0xff0000).wrapping_shr(16).rem(256) as u8);
                bitstream_ptr
                    .add(BCD_TICKS_OFFSET + 3)
                    .write((bcd_ticks & 0xff000000).wrapping_shr(24).rem(256) as u8);
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

    fn init_pins(pins: Pins, delay: &mut Delay) -> Result<(), Error> {
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
                    data.set_high()?;
                } else {
                    data.set_low()?;
                }

                delay.delay_us(10);
                clock.set_high()?;
                delay.delay_us(10);
                clock.set_low()?;
            }
        }

        for i in 0..16 {
            if reg1 & (1 << (15 - i)) != 0 {
                data.set_high()?;
            } else {
                data.set_low()?;
            }

            delay.delay_us(10);
            clock.set_high()?;
            delay.delay_us(10);
            clock.set_low()?;

            if i == 4 {
                latch.set_high()?;
            }
        }

        latch.set_low()?;

        // reapply the blank as the above seems to cause a slight glow
        blank.set_low()?;
        delay.delay_us(10);
        blank.set_high()?;

        Ok(())
    }

    pub fn new(resources: CosmicBuilder) -> Self {
        let CosmicBuilder {
            pins,
            mut delay,
            mut pio,
            sm0,
            dma,
        } = resources;

        Self::init_pins(pins, &mut delay).expect("CANNOT INITIALIZE BITSTREAM");

        let program = pio_proc::pio_file!("cosmic_unicorn.pio");
        let installed = pio.install(&program.program).unwrap();

        let (mut sm, _, tx) = PIOBuilder::from_installed_program(installed)
            .out_pins(17, 4)
            .set_pins(14, 3)
            .side_set_pin_base(13)
            .autopull(true)
            .pull_threshold(32)
            .buffers(Buffers::OnlyTx)
            .build(sm0);

        sm.set_pindirs((13..=20).zip(core::iter::repeat(PinDir::Output)));

        unsafe {
            Self::init_bitstream(&mut BITSTREAM);
        }

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

        let mut x = (WIDTH - 1) - x;
        let mut y = (HEIGHT - 1) - y;

        if y < 16 {
            x += 32;
        } else {
            y -= 16;
        }

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
