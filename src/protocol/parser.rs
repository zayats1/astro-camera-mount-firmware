use core::fmt;

use super::{
    comands::{
        ANGLE_PREFIX, EQ_VAL, SEPPARATOR, STEPPER_SPEED_PREFIX, STEPPER_STEPS_PREFIX, STEPPER_STOP,
    },
    message::Message,
};

#[derive(PartialEq)]
pub enum ParsingError {
    SepparatorError,
    ValueParsingError,
    NotAComandError,
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Can't parse the message")
    }
}

pub fn parse(signal: &str) -> Result<Message, ParsingError> {
    let Some(sep_idx) = get_sepparator_index(signal, SEPPARATOR) else {
        return Err(ParsingError::SepparatorError);
    };
    let cut_part_1 = &signal[..sep_idx];

    let Some(val_sep_idx) = get_sepparator_index(cut_part_1, EQ_VAL) else {
        return Err(ParsingError::SepparatorError);
    };

    let comand = &cut_part_1[..val_sep_idx];
    let value = &cut_part_1[val_sep_idx + 1..];

    match comand {
        ANGLE_PREFIX => {
            if let Ok(angle) = value.parse::<f32>() {
                Ok(Message::ServoAngle(angle))
            } else {
                Err(ParsingError::ValueParsingError)
            }
        }
        STEPPER_STEPS_PREFIX => {
            if let Ok(steps) = value.parse::<i32>() {
                Ok(Message::StepperMotorRunSteps(steps))
            } else {
                Err(ParsingError::ValueParsingError)
            }
        }

        STEPPER_SPEED_PREFIX => {
            if let Ok(speed) = value.parse::<f32>() {
                Ok(Message::StepperMotorSpeed(speed))
            } else {
                Err(ParsingError::ValueParsingError)
            }
        }
        STEPPER_STOP => Ok(Message::StepperStop),
        _ => Err(ParsingError::NotAComandError),
    }
}

fn get_sepparator_index(string: &str, sepparator: char) -> Option<usize> {
    let mut sep_index = None;
    for (i, ch) in string.chars().enumerate() {
        if ch == sepparator {
            sep_index = Some(i);
            break;
        }
    }
    sep_index
}
