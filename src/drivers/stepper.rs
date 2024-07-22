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
            direction: Direction::default(),
            speed: 2.0,
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub async fn steps<F, Fut>(&mut self, steps: i32, delay: F)
    where
        F: Fn(u64) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut steps = steps;
        if steps > 0 {
            self.set_dir(Direction::Forward);
        } else {
            self.set_dir(Direction::Backward);
            steps *= -1;
        }

        for _ in 0..steps {
            if self.direction == Direction::Stop {
                break;
            }
            if self.speed > 0.0 {
                let delay_val_ms = (1000.0 / self.speed) as u64;
                self.clk_pin.set_high().unwrap_or_default();
                delay(delay_val_ms).await;
                self.clk_pin.set_low().unwrap_or_default();
                delay(delay_val_ms).await;
            } else {
                break;
            }
        }
    }
    pub fn set_dir(&mut self, dir: Direction) {
        self.direction = dir;
        match self.direction {
            Direction::Forward => self.dir_pin.set_high().unwrap_or_default(),
            Direction::Backward => self.dir_pin.set_low().unwrap_or_default(),
            Direction::Stop => (),
        }
    }
}
