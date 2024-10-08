//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
#![allow(static_mut_refs)]
#![feature(ptr_to_from_bits)]
#![feature(strict_provenance)]
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
use rp_pico::{entry, hal::pio::PIOExt};

use rp_pico::hal::dma::DMAExt;
use rp_pico::hal::{
    clocks::{init_clocks_and_plls, Clock},
    fugit::{self},
    gpio::Pins,
    pac,
    watchdog::Watchdog,
    Sio,
};

mod builder;
mod constants;
mod cosmic_unicorn;
mod framebuffer;
mod pixel;
mod sketch;

use cosmic_unicorn::CosmicUnicorn;
use framebuffer::FrameBuffer;

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let (pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let dma = pac.DMA.split(&mut pac.RESETS);
    let delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let builder = builder::CosmicBuilder {
        pins,
        delay,
        pio,
        sm0,
        dma,
    };

    info!("Got Peripherals, initializing LED matrix");

    watchdog.start(fugit::Duration::<u32, 1, 1000000>::secs(1));

    let cosmic_unicorn = CosmicUnicorn::new(builder);
    info!("Matrix ready, initializing sketch");

    info!("Enter main loop:");
    let red = &[pixel::Pixel::new(255, 1, 1); constants::WIDTH * constants::HEIGHT];
    let blue = &[pixel::Pixel::new(1, 1, 255); constants::WIDTH * constants::HEIGHT];
    let mut counter = 0usize;
    let mut is_red = false;

    loop {
        watchdog.feed();
        cosmic_unicorn.update(if is_red { red } else { blue });
        if counter % 300 == 0 {
            is_red = !is_red;
        }
        counter += 1;
    }
}
