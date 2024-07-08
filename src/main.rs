#![no_std]
#![no_main]

mod drivers;
mod protocol;
mod tests;

use panic_halt as _;
use rtic_monotonics::rp2040::prelude::*;
rp2040_timer_monotonic!(Mono);

#[rtic::app(device = rp_pico::hal::pac)]
mod app {
    use super::*;
    use drivers::stepper::{self};

    use rp_pico::{
        hal::{
            clocks::{init_clocks_and_plls, Clock},
            gpio::{
                self,
                bank0::{Gpio0, Gpio1},
            },
            pac::{self},
            sio::Sio,
            uart::{self, DataBits, StopBits, UartConfig},
            watchdog::Watchdog,
        },
        Pins,
    };

    use crate::{
        drivers::stepper::{Direction, StepperWithDriver},
        tests::tester::Tester,
    };
    use embedded_hal_nb::serial::{Read, Write};

    // Time handling traits
    use fugit::RateExtU32;

    /// Alias the type for our UART pins to make things clearer.
    type UartPins = (
        gpio::Pin<Gpio0, gpio::FunctionUart, gpio::PullNone>,
        gpio::Pin<Gpio1, gpio::FunctionUart, gpio::PullNone>,
    );

    type Stepper = StepperWithDriver<
        gpio::Pin<gpio::bank0::Gpio25, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
        gpio::Pin<gpio::bank0::Gpio15, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    >;
    /// Alias the type for our UART to make things clearer.
    type Uart = uart::UartPeripheral<uart::Enabled, pac::UART0, UartPins>;

    #[shared]
    struct Shared {}

    // Local resources to specific tasks (cannot be shared)
    #[local]
    struct Local {
        uart: Uart,
        stepper: Stepper,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut pac = ctx.device;
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

        let mut tester = Tester::new(&mut uart);
        tester.run_tests();

        rtic::pend(pac::Interrupt::UART0_IRQ);
        // Tell the UART to raise its interrupt line on the NVIC when the RX FIFO
        // has data in it.
        Mono::start(pac.TIMER, &pac.RESETS);
        uart.enable_rx_interrupt();

        if let Err(_) = main_task::spawn() {
            uart.write_full_blocking(b"Error, can`t spawn the task");
            panic!();
        }
        (
            // Initialization of shared resources
            Shared {},
            // Initialization of task local resources
            Local { uart, stepper },
        )
    }

    #[task(local = [stepper])]
    async fn main_task(ctx: main_task::Context) {
        loop {
            ctx.local.stepper.step();
            Mono::delay(500.millis()).await;
            ctx.local.stepper.step();
            Mono::delay(500.millis()).await;
        }
    }
    #[task(binds = UART0_IRQ, local = [uart])]
    fn uart0_task(ctx: uart0_task::Context) {
        let uart = ctx.local.uart;

        while let Ok(byte) = uart.read() {
            let _ = uart.write(byte);
        }
        cortex_m::asm::sev();
    }
}
// End of file
