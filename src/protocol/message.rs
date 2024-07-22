#[derive(PartialEq)]
pub enum Message {
    StepperMotorRunSteps(i32),
    StepperMotorSpeed(f32),
    ServoAngle(f32),
    StepperStop,
}
