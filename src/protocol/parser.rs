use core::fmt;

use super::{
    message::Message,
    protocol::{ANGLE_PREFIX, EQ_VAL, SEPPARATOR, STEPPER_STEPS_PREFIX, STEPPER_STOP},
};

#[derive(PartialEq)]
pub struct ParsingError;

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Can't parse tthe message")
    }
}

pub fn parse(signal: &str) -> Result<Message, ParsingError> {
    if let Some(sep_idx) = get_sepparator_index(signal, SEPPARATOR) {
        let cut_part_1 = &signal[..sep_idx];

        if let Some(val_sep_idx) = get_sepparator_index(cut_part_1, EQ_VAL) {
            let comand = &cut_part_1[..val_sep_idx];
            let value = &cut_part_1[val_sep_idx + 1..];

            match comand {
                ANGLE_PREFIX => {
                    if let Some(angle) = value.parse::<f32>().ok() {
                        return Ok(Message::ServoAngle(angle));
                    }
                }
                STEPPER_STEPS_PREFIX => {
                    if let Some(steps) = value.parse::<i32>().ok() {
                        return Ok(Message::StepperMotorRunSteps(steps));
                    }
                }
                STEPPER_STOP => {
                    return Ok(Message::StepperStop);
                }
                _ => (),
            }
        }
    }
    Err(ParsingError)
}

fn get_sepparator_index(string: &str, sepparator: char) -> Option<usize> {
    let mut sep_index = None;
    for (i, ch) in string.chars().enumerate() {
        if ch == sepparator {
            sep_index = Some(i);
            break;
        }
    }
    return sep_index;
}
