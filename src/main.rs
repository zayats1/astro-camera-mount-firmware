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

    use heapless::Vec;
    use protocol::{
        message::{self, Message},
        parser::parse,
    };
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

    use rtic_sync::channel::{Receiver, Sender};
    use rtic_sync::make_channel;

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

    const MESSAGES: usize = 1;
    // Local resources to specific tasks (cannot be shared)
    #[local]
    struct Local {
        uart: Uart,
        stepper: Stepper,
        sender: Sender<'static, Message, MESSAGES>,
        receiver: Receiver<'static, Message, MESSAGES>,
    }

    const CAPACITY: usize = 16;
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

        let (sender, receiver) = make_channel!(Message, MESSAGES);
        // has data in it.
        Mono::start(pac.TIMER, &pac.RESETS);
        uart.enable_rx_interrupt();

        main_task::spawn().unwrap();
        (
            // Initialization of shared resources
            Shared {},
            // Initialization of task local resources
            Local {
                uart,
                stepper,
                sender,
                receiver,
            },
        )
    }

    #[task(local = [receiver])]
    async fn main_task(ctx: main_task::Context) {
        let reciever = ctx.local.receiver;
        loop {
            if let Ok(message) = reciever.recv().await {
                match message {
                    Message::StepperMotorRunSteps(steps) => stepper_steps::spawn(steps).unwrap(),
                    Message::StepperMotorSpeed(_) => todo!(),
                    Message::ServoAngle(_) => todo!(),
                    Message::StepperStop => todo!(),
                }
            }
        }
    }

    #[task(local = [stepper])]
    async fn stepper_steps(ctx: stepper_steps::Context, steps: i32) {
        let stepper = ctx.local.stepper;
        let delay = |time: u64| Mono::delay(time.millis());
        let speed = 2.0;
        let delay_val_ms = (1000.0 / speed) as u64;

        // step has two phases
        for _ in 0..steps * 2 {
            stepper.step();
            delay(delay_val_ms).await;
        }
    }

    #[task(binds = UART0_IRQ, local = [uart,sender])]
    fn uart0_task(ctx: uart0_task::Context) {
        let uart = ctx.local.uart;
        let sender = ctx.local.sender;

        let mut data = Vec::<u8, CAPACITY>::new();
        while let Ok(byte) = uart.read() {
            // Todo: a proper error handling
            uart.write(byte).ok();
            if let Err(_) = data.push(byte) {
                break;
            }
        }

        if let Ok(data_string) = core::str::from_utf8(&data) {
            if let Ok(message) = parse(data_string) {
                sender.try_send(message).ok();
            }
        }
        data.clear();
        cortex_m::asm::sev();
    }
}
// End of file
