use core::future::Future;

use embedded_hal::digital::OutputPin;

#[derive(Default, PartialEq)]
pub enum Direction {
    Forward,
    Backward,
    #[default]
    Stop,
}

pub struct StepperWithDriver<T: OutputPin, U: OutputPin> {
    clk_pin: T,
    dir_pin: U,
    step_phase: bool,
    speed: f32,
    direction: Direction,
}

impl<T, U> StepperWithDriver<T, U>
where
    T: OutputPin,
    U: OutputPin,
{
    pub fn new(clk_pin: T, dir_pin: U) -> Self {
        Self {
            clk_pin,
            dir_pin,
            step_phase: false,
            direction: Direction::default(),
            speed: 2.0,
        }
    }
    pub fn step(&mut self) {
        if self.direction != Direction::Stop {
            if self.step_phase {
                self.clk_pin.set_high().unwrap_or_default();
                self.step_phase = false;
            } else {
                self.clk_pin.set_low().unwrap_or_default();
                self.step_phase = true;
            }
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        let abs_speed = f32::from_bits(speed.to_bits() & i32::MAX as u32);
        self.speed = abs_speed;
    }

    pub async fn steps<F, Fut>(&mut self, steps: i32, delay: F)
    where
        F: Fn(u64) -> Fut,
        Fut: Future<Output = ()>,
    {
        if self.speed != 0.0 {
            let delay_val_ms = (1000.0 / self.speed) as u64;

            // step has two phases
            for _ in 0..steps * 2 {
                self.step();
                delay(delay_val_ms).await;
            }
        }
    }
    pub fn set_dir(&mut self, dir: Direction) {
        self.direction = dir;
        match self.direction {
            Direction::Forward => self.dir_pin.set_high().unwrap_or_default(),
            Direction::Backward => self.dir_pin.set_low().unwrap_or_default(),
            Direction::Stop => self.speed = 0.0,
        }
    }
}
