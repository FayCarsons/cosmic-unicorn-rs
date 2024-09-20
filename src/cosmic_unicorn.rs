use core::ops::Rem;

use cortex_m::delay::Delay;
use embedded_hal::digital::OutputPin;

use hal::pio::{Buffers, PinDir};
use rp_pico::hal;
use rp_pico::hal::dma::{Channel, CH0, CH1};
use rp_pico::hal::pio::{Running, StateMachine, Tx, SM0};
use rp_pico::pac::PIO0;

use crate::builder::CosmicBuilder;
use hal::dma::SingleChannel;
use hal::gpio::{Error, PinState, Pins};
use hal::pio::PIOBuilder;

use super::constants::*;
use super::framebuffer::FrameBuffer;

use defmt::info;

type Buffer = [u8; BITSTREAM_LENGTH];

#[repr(C, align(4))]
struct BitStream(Buffer);

impl BitStream {
    fn get_mut(&mut self) -> &mut Buffer {
        &mut self.0
    }

    const fn get(&self) -> Buffer {
        self.0
    }
}

#[allow(unused)]
pub struct CosmicUnicorn {
    state_machine: StateMachine<(PIO0, SM0), Running>,
    tx: Tx<(PIO0, SM0)>,
    dma_transfer_channel: Channel<CH0>,
    dma_control_channel: Channel<CH1>,
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

static mut BITSTREAM: BitStream = BitStream([0; BITSTREAM_LENGTH]);

impl CosmicUnicorn {
    unsafe fn init_bitstream(bitstream: &mut BitStream) {
        for row in 0..16 {
            for frame in 0..BCD_FRAME_COUNT {
                let offset = row * ROW_BYTES + (BCD_FRAME_BYTES * frame);
                let bitstream_ptr = bitstream.get_mut().as_mut_ptr().add(offset);

                bitstream_ptr.write(63);
                bitstream_ptr.add(1).write(row as u8);

                let bcd_ticks = 1u32 << frame;
                bitstream_ptr
                    .add(68)
                    .write((bcd_ticks & 0xff).rem(256) as u8);
                bitstream_ptr
                    .add(69)
                    .write((bcd_ticks & 0xff00).wrapping_shr(8).rem(256) as u8);
                bitstream_ptr
                    .add(70)
                    .write((bcd_ticks & 0xff0000).wrapping_shr(16).rem(256) as u8);
                bitstream_ptr
                    .add(71)
                    .write((bcd_ticks & 0xff000000).wrapping_shr(24).rem(256) as u8);
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

        let reg1 = 0b1111111111001110u16;

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

        unsafe {
            Self::init_bitstream(&mut BITSTREAM);
        }

        match Self::init_pins(pins, &mut delay) {
            Ok(_) => info!("Initialized pins and sent init message to led matrix!"),
            Err(_) => defmt::panic!("CANNOT INITIALIZE BITSTREAM"),
        };

        let program = pio_proc::pio_file!("cosmic_unicorn.pio");
        let installed = pio.install(&program.program).unwrap();
        info!("Compiled and installed PIO program");

        let (mut sm, _, tx) = PIOBuilder::from_installed_program(installed)
            .out_pins(17, 4)
            .set_pins(14, 3)
            .side_set_pin_base(13)
            .autopull(true)
            .pull_threshold(32)
            .buffers(Buffers::OnlyTx)
            .clock_divisor_fixed_point(1, 0)
            .build(sm0);

        sm.set_pins((16..=20).zip(core::iter::repeat(rp_pico::hal::pio::PinState::High)));
        sm.set_pindirs((13..=20).zip(core::iter::repeat(PinDir::Output)));
        info!("PIO ready");

        info!("Bitstream ready");

        info!("PIO state machine started");

        let ch0 = dma.ch0.ch();
        let ch1 = dma.ch1.ch();
        ch0.ch_ctrl_trig().write(|reg| unsafe {
            reg.data_size().size_word();
            reg.incr_read().set_bit();
            reg.incr_write().clear_bit();
            reg.treq_sel().bits(tx.dreq_value());
            reg.chain_to().bits(1);
            reg.irq_quiet().set_bit()
        });

        ch0.ch_read_addr()
            .write(|reg| unsafe { reg.bits(BITSTREAM.get().as_mut_ptr().addr() as u32) });

        ch0.ch_write_addr()
            .write(|reg| unsafe { reg.bits(tx.fifo_address().addr() as u32) });

        ch0.ch_trans_count()
            .write(|reg| unsafe { reg.bits((BITSTREAM_LENGTH / 4) as u32) });

        ch1.ch_ctrl_trig().write(|reg| unsafe {
            reg.data_size().size_word();
            reg.incr_read().clear_bit();
            reg.incr_write().clear_bit();
            reg.chain_to().bits(0);
            reg.ring_size().bits(2);
            reg.ring_sel().set_bit()
        });

        ch1.ch_read_addr()
            .write(|reg| unsafe { reg.bits(BITSTREAM.get().as_mut_ptr().addr() as u32) });

        ch1.ch_write_addr()
            .write(|reg| unsafe { reg.bits(ch0.ch_read_addr().as_ptr().addr() as u32) });

        ch1.ch_trans_count().write(|reg| unsafe { reg.bits(1) });

        let state_machine = sm.start();
        ch1.ch_ctrl_trig().write(|reg| reg.en().set_bit());
        ch0.ch_ctrl_trig().write(|reg| reg.en().set_bit());
        info!("DMA transfer started");

        Self {
            state_machine,
            tx,
            dma_transfer_channel: dma.ch0,
            dma_control_channel: dma.ch1,
        }
    }

    fn set_pixel(&self, mut x: usize, mut y: usize, pixel: [u8; 3]) {
        x = (WIDTH - 1) - x;
        y = (HEIGHT - 1) - y;

        if y < 16 {
            x += 32;
        } else {
            y -= 16;
        }

        // The brightness adjustment has not been implemented, so we ignore that
        // portion of the original firmware
        let [mut r, mut g, mut b] = pixel.map(|c| GAMMA[c.wrapping_shr(8) as usize]);

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

        // set the appropriate bits in the separate bcd frames
        for frame in 0..BCD_FRAME_COUNT {
            let offset = y * ROW_BYTES + (BCD_FRAME_BYTES * frame) + 2 + x;

            let red_bit = r & 0b1;
            let green_bit = g & 0b1;
            let blue_bit = b & 0b1;

            let pixel = (blue_bit | (green_bit << 1) | (red_bit << 2)) as u8;
            unsafe {
                BITSTREAM
                    .get_mut()
                    .as_mut_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .write(pixel);
            }

            r >>= 1;
            g >>= 1;
            b >>= 1;
        }
    }

    pub fn update<B>(&self, buffer: &B)
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
                let b = col & 0x0000ff;
                self.set_pixel(x, y, [r as u8, g as u8, b as u8]);
                offset += 1;
            }
        }
    }
}
