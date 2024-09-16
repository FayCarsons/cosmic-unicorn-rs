use cortex_m::delay::Delay;
use rp_pico::{
    hal::{
        dma::Channels,
        pio::{UninitStateMachine, PIO, SM0},
    },
    pac::PIO0,
};

use rp_pico::hal::gpio::Pins;

pub struct CosmicBuilder {
    pub pins: Pins,
    pub delay: Delay,
    pub pio: PIO<PIO0>,
    pub sm0: UninitStateMachine<(PIO0, SM0)>,
    pub dma: Channels,
}
