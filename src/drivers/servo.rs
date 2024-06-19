use embedded_hal::pwm::{self, SetDutyCycle};
struct Servo<'a, T: SetDutyCycle> {
    pwm_out: &'a mut T,
    duty_on_zero: u16,
    duty_on_90: u16,
    duty_per_degree: f32,
    max_angle: u16,
}

impl<'a, T: SetDutyCycle> Servo<'a, T> {
    pub fn new(pwm_out: &'a mut T, duty_on_zero: u16, max_angle: u16) -> Self {
        let duty_on_90 = duty_on_zero * 3;
        let duty_per_degree = (duty_on_90 - duty_on_zero) as f32 / 90.0;
        Self {
            pwm_out,
            duty_on_zero,
            duty_on_90,
            duty_per_degree,
            max_angle,
        }
    }

    pub fn set_angle(&mut self, angle: u16) {
        let mut angle = angle;

        if angle > self.max_angle {
            angle = self.max_angle
        }

        let duty_on_the_degree = (self.duty_per_degree * angle as f32) as u16 + self.duty_on_zero;

        self.pwm_out.set_duty_cycle(duty_on_the_degree).unwrap();
    }
}
