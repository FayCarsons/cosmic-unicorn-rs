//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use cortex_m::asm;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
use rp_pico::{entry, hal::pio::PIOExt};

use rp_pico::hal::dma::DMAExt;
use rp_pico::hal::{
    clocks::{init_clocks_and_plls, Clock},
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

    let cosmic_unicorn = CosmicUnicorn::new(builder);

    loop {
        asm::nop()
    }
}
