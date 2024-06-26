//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

mod drivers;
mod protocol;

use defmt::*;
use defmt_rtt as _;
use drivers::stepper::{self, *};
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
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

use embedded_hal_nb::serial::{Read, Write};

use core::cell::RefCell;
use critical_section::Mutex;

/// Alias the type for our UART pins to make things clearer.
type UartPins = (
    gpio::Pin<Gpio0, gpio::FunctionUart, gpio::PullNone>,
    gpio::Pin<Gpio1, gpio::FunctionUart, gpio::PullNone>,
);

/// Alias the type for our UART to make things clearer.
type Uart = uart::UartPeripheral<uart::Enabled, pac::UART0, UartPins>;

/// This how we transfer the UART into the Interrupt Handler
static GLOBAL_UART: Mutex<RefCell<Option<Uart>>> = Mutex::new(RefCell::new(None));
// Time handling traits
use fugit::RateExtU32;

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

    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    //
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead.
    // One way to do that is by using [embassy](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/wifi_blinky.rs)
    //
    // If you have a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here. Don't forget adding an appropriate resistor
    // in series with the LED.
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
    // This variable is special. It gets mangled by the `#[interrupt]` macro
    // into something that we can access without the `unsafe` keyword. It can
    // do this because this function cannot be called re-entrantly. We know
    // this because the function's 'real' name is unknown, and hence it cannot
    // be called from the main thread. We also know that the NVIC will not
    // re-entrantly call an interrupt.
    static mut UART: Option<uart::UartPeripheral<uart::Enabled, pac::UART0, UartPins>> = None;

    // This is one-time lazy initialisation. We steal the variable given to us
    // via `GLOBAL_UART`.
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
