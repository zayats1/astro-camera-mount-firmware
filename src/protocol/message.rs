#[derive(PartialEq)]
pub enum Message {
    StepperMotorRunSteps(i32),
    ServoAngle(f32),
    StepperStop,
}
