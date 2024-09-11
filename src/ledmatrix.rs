use pio::ProgramWithDefines;

use rp_pico::hal;

use hal::pio::{ PIOBuilder, StateMachine, UninitStateMachine, PIO, SM0};
use hal::dma::{Channel, CH0};
use hal::gpio::bank0::{Gpio13, Gpio14, Gpio15, Gpio16, Gpio17, Gpio18, Gpio19, Gpio20, Pin, FunctionPio0, FunctionNull, PullUp, PullNone};

pub const WIDTH: usize = 32;
pub const HEIGHT: usize = 32;
const ROW_COUNT: usize = 16;
const BCD_FRAME_COUNT: usize = 14;
const BCD_FRAME_BYTES: usize = 72;
const ROW_BYTES: usize = BCD_FRAME_COUNT * BCD_FRAME_BYTES;
const BITSTREAM_LENGTH: usize = ROW_COUNT * ROW_BYTES;

const SYSTEM_FREQUENCY: u32 = 22_050;

struct Column {
    clock: Pin<Gpio13, FunctionPio0, PullUp>,
    data: Pin<Gpio14, FunctionPio0, PullUp>,
    latch: Pin<Gpio15, FunctionPio0, PullUp>,
    _blank: Pin<Gpio16, FunctionNull, PullNone>,
}

struct Row(
    Pin<Gpio17, FunctionPio0, PullUp>,
    Pin<Gpio18, FunctionPio0, PullUp>,
    Pin<Gpio19, FunctionPio0, PullUp>,
    Pin<Gpio20, FunctionPio0, PullUp>,
);

type Stream = [u8; BITSTREAM_LENGTH];
static mut BITSTREAM: Stream = [0; BITSTREAM_LENGTH];
struct PioManager {
    bit_stream_state_machine: SM0,
    bit_stream_offset: u32,
    stream: *mut u8
}

use rp_pico::hal::pio::PIOExt;

impl PioManager {
    unsafe fn init_stream() {
        for row in 0..16 {
            for frame in 0..BCD_FRAME_COUNT {
                let offset = row * ROW_BYTES + frame * BCD_FRAME_BYTES;
                
                let bitstream_ptr = BITSTREAM.as_mut_ptr().offset(offset as isize);
                bitstream_ptr.write(63);

                bitstream_ptr.offset(1).write(row as u8);

                let bcd_ticks = 1 << frame;
                bitstream_ptr.offset(68).write((bcd_ticks & 0xff) >> 0);
                bitstream_ptr.offset(69).write((bcd_ticks & 0xff00) >> 8);
                bitstream_ptr.offset(70).write((bcd_ticks & 0xff0000) >> 16);
                bitstream_ptr.offset(71).write((bcd_ticks & 0xff000000) >> 24);
            }
        }
    }


    fn init(pac: rp_pico::pac::Peripherals) -> Self {
        let program = pio_proc::pio_file!("cosmic_unicorn.pio");
        let (mut pio, sm0, a, b, c) = pac.PIO0.split(&mut pac.RESETS);
        let installed = pio.install(&program.program).unwrap();

        let (mut sm, _, _) = PIOBuilder::from_installed_program(installed)
            .out_pins(3, 4)           // out pins: starting at 3, using 4 pins (3, 4, 5, 6 for row select)
            .set_pins(0, 3)           // set pins: 0 (column data), 1 (column latch), 2 (column blank)
            .side_set_pin_base(0)     // sideset pin: 0 for column clock
            .out_shift_direction(hal::pio::ShiftDirection::Right)
            .in_shift_direction(hal::pio::ShiftDirection::Right)
            .autopull(true)
            .pull_threshold(32)       // 32 bits (4 bytes) per pull
            .clock_divisor_fixed_point(1, 0)  // Run at full system clock
            .build(sm0);

        sm.start();

        let stream = unsafe { BITSTREAM.as_mut_ptr() };
    }
}

pub struct LedMatrix {
    dma_channel: CH0,
    dma_control_channel: u32,
    column: Column,
    row: Row,
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

impl LedMatrix {
    fn init_stream()

    pub fn new(pac: rp_pico::pac::Peripherals) -> Self {
    }

    #[inline]
    pub fn pio_program_init(pio: PIO<PIO0>, sm: SM0, offset: u32) {}

    fn dma_complete() {}

    pub fn set_pixel(x: usize, y: usize, pixel: Pixel) {

    }
}
