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
    use drivers::{
        servo::Servo,
        stepper::{self},
    };

    use heapless::Vec;
    use protocol::{message::Message, parser::parse};
    use rp_pico::{
        Pins,
        hal::{
            self,
            clocks::{Clock, init_clocks_and_plls},
            gpio::{
                self,
                bank0::{Gpio0, Gpio1, Gpio16, Gpio17},
            },
            pac::{self},
            sio::Sio,
            uart::{self, DataBits, StopBits, UartConfig},
            watchdog::Watchdog,
        },
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
        gpio::Pin<Gpio16, gpio::FunctionUart, gpio::PullNone>,
        gpio::Pin<Gpio17, gpio::FunctionUart, gpio::PullNone>,
    );

    type Stepper = StepperWithDriver<
        gpio::Pin<gpio::bank0::Gpio15, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
        gpio::Pin<gpio::bank0::Gpio14, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    >;
    /// Alias the type for our UART to make things clearer.
    type Uart = uart::UartPeripheral<uart::Enabled, pac::UART0, UartPins>;
    type MyServo = Servo<
        hal::pwm::Channel<hal::pwm::Slice<hal::pwm::Pwm1, hal::pwm::FreeRunning>, hal::pwm::B>,
    >;

    #[shared]
    struct Shared {}

    const MESSAGES: usize = 1;
    // Local resources to specific tasks (cannot be shared)
    #[local]
    struct Local {
        uart: Uart,
        stepper: Stepper,
        sender: Sender<'static, Message, MESSAGES>,
        servo: MyServo,
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

        let led_pin = pins.gpio15.into_push_pull_output();
        let dir_pin = pins.gpio14.into_push_pull_output();

        let mut stepper = stepper::StepperWithDriver::new(led_pin, dir_pin);
        stepper.set_dir(Direction::Forward);

        let uart_pins = (
            // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
            pins.gpio16.reconfigure(),
            // UART RX (characters received by RP2040) on pin 2 (GPIO1)
            pins.gpio17.reconfigure(),
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

        let mut servo_pwm = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS).pwm1;

        let pwm_period = 20u8; //20ms or 50 hz
        servo_pwm.set_ph_correct();
        servo_pwm.set_div_int(pwm_period);
        servo_pwm.enable();

        let mut channel = servo_pwm.channel_b;
        channel.output_to(pins.gpio3);

        let servo_max_angle = 180.0;
        let servo = Servo::new(channel, pwm_period, servo_max_angle);

        main_task::spawn().unwrap();
        (
            // Initialization of shared resources
            Shared {},
            // Initialization of task local resources
            Local {
                uart,
                stepper,
                sender,
                servo,
                receiver,
            },
        )
    }

    #[task(local = [receiver,servo,stepper])]
    async fn main_task(ctx: main_task::Context) {
        let reciever = ctx.local.receiver;
        let servo = ctx.local.servo;
        let stepper = ctx.local.stepper;
        loop {
            if let Ok(message) = reciever.recv().await {
                match message {
                    Message::StepperMotorRunSteps(steps) => {
                        if stepper_steps::spawn(stepper, steps).is_err() {
                            continue;
                        }
                    }
                    Message::StepperMotorSpeed(speed) => stepper.set_speed(speed),
                    Message::ServoAngle(angle) => servo.set_angle(angle),
                    Message::StepperStop => stepper.set_dir(Direction::Stop),
                }
            }
        }
    }

    #[task()]
    async fn stepper_steps(_ctx: stepper_steps::Context, stepper: &mut Stepper, steps: i32) {
        let delay = |time: u64| Mono::delay(time.millis());
        stepper.steps(steps, delay).await;
    }

    #[task(binds = UART0_IRQ, local = [uart,sender])]
    fn uart0_task(ctx: uart0_task::Context) {
        let uart = ctx.local.uart;
        let sender = ctx.local.sender;

        let mut data = Vec::<u8, CAPACITY>::new();
        while let Ok(byte) = uart.read() {
            // Todo: a proper error handling
            uart.write(byte).ok();
            if data.push(byte).is_err() {
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
