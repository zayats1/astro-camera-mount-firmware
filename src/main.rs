#![no_std]
#![no_main]

mod drivers;
mod protocol;
mod tests;

use defmt::*;
use defmt_rtt as _;
use drivers::stepper::{self, *};
use panic_probe as _;

use rp_pico::{
    entry,
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::{
            self,
            bank0::{Gpio0, Gpio1},
        },
        pac::{self, interrupt},
        sio::Sio,
        uart::{self, DataBits, StopBits, UartConfig},
        watchdog::Watchdog,
    },
    Pins,
};

use crate::tests::tester::Tester;
use embedded_hal_nb::serial::{Read, Write};

use core::cell::RefCell;
use critical_section::Mutex;

// Time handling traits
use fugit::RateExtU32;

/// Alias the type for our UART pins to make things clearer.
type UartPins = (
    gpio::Pin<Gpio0, gpio::FunctionUart, gpio::PullNone>,
    gpio::Pin<Gpio1, gpio::FunctionUart, gpio::PullNone>,
);

/// Alias the type for our UART to make things clearer.
type Uart = uart::UartPeripheral<uart::Enabled, pac::UART0, UartPins>;

/// This how we transfer the UART into the Interrupt Handler
static GLOBAL_UART: Mutex<RefCell<Option<Uart>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let led_pin = pins.led.into_push_pull_output();
    let dir_pin = pins.gpio15.into_push_pull_output();

    let mut stepper = stepper::StepperWithDriver::new(led_pin, dir_pin);
    stepper.set_dir(Direction::Forward);

    let uart_pins = (
        // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
        pins.gpio0.reconfigure(),
        // UART RX (characters received by RP2040) on pin 2 (GPIO1)
        pins.gpio1.reconfigure(),
    );

    // Make a UART on the given pins
    let mut uart = uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    unsafe {
        // Enable the UART interrupt in the *Nested Vectored Interrupt
        // Controller*, which is part of the Cortex-M0+ core.
        pac::NVIC::unmask(pac::Interrupt::UART0_IRQ);
    }

    // Tell the UART to raise its interrupt line on the NVIC when the RX FIFO
    // has data in it.
    uart.enable_rx_interrupt();

    // Write something to the UART on start-up so we can check the output pin
    // is wired correctly.
    uart.write_full_blocking(b"uart_interrupt example started...\n");

    let mut tester = Tester::new(&mut uart);
    tester.run_tests();
    critical_section::with(|cs| {
        GLOBAL_UART.borrow(cs).replace(Some(uart));
    });

    loop {
        info!("on!");
        stepper.step();
        delay.delay_ms(500);
        info!("off!");
        stepper.step();
        delay.delay_ms(500);
    }
}

#[interrupt]
#[allow(non_snake_case)]
fn UART0_IRQ() {
    static mut UART: Option<uart::UartPeripheral<uart::Enabled, pac::UART0, UartPins>> = None;

    if UART.is_none() {
        critical_section::with(|cs| {
            *UART = GLOBAL_UART.borrow(cs).take();
        });
    }

    // Check if we have a UART to work with
    if let Some(uart) = UART {
        // Echo the input back to the output until the FIFO is empty. Reading
        // from the UART should also clear the UART interrupt flag.
        while let Ok(byte) = uart.read() {
            let _ = uart.write(byte);
        }
    }

    // Set an event to ensure the main thread always wakes up, even if it's in
    // the process of going to sleep.
    cortex_m::asm::sev();
}

// End of file
