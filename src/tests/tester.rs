use core::fmt::Write;

use crate::protocol::{
    message::Message,
    parser::{parse, ParsingError},
};

pub struct Tester<'a, T: Write> {
    tx: &'a mut T,
}

impl<'a, T> Tester<'a, T>
where
    T: Write,
{
    pub fn new(tx: &'a mut T) -> Self {
        Self { tx }
    }
    fn parse_servo_test(&mut self) {
        let message = "ANGLE:90,";
        let res = parse(message);
        self.assert_eq(res, Ok(Message::ServoAngle(90.0)));

        self.tx.write_str("PASSED \n").unwrap();
    }

    fn parse_stepper_test(&mut self) {
        let message = "STEPS:25,";
        let res = parse(message);
        self.assert_eq(res, Ok(Message::StepperMotorRunSteps(25)));

        let message = "STOP:,";
        let res = parse(message);
        self.assert_eq(res, Ok(Message::StepperStop));

        self.tx.write_str("PASSED \n").unwrap();
    }

    fn parse_value_error_test(&mut self) {
        let message = "STEPS:,";
        let res = parse(message);
        self.assert_eq(res, Err(ParsingError::ValueParsingError));
        self.tx.write_str("PASSED \n").unwrap();
    }
    fn parse_not_a_comand_error(&mut self) {
        let message = "STO:,";
        let res = parse(message);
        self.assert_eq(res, Err(ParsingError::NotAComandError));
        self.tx.write_str("PASSED \n").unwrap();
    }

    fn parse_sepparator_error(&mut self) {
        let message = "STEPS4,";
        let res = parse(message);
        self.assert_eq(res, Err(ParsingError::SepparatorError));

        let message = "STEPS:4";
        let res = parse(message);
        self.assert_eq(res, Err(ParsingError::SepparatorError));

        self.tx.write_str("PASSED \n").unwrap();
    }

    fn assert_eq<U: PartialEq>(&mut self, res: U, expected: U) {
        if res != expected {
            self.tx.write_str("FAILED").unwrap();
            panic!();
        }
    }

    pub fn run_tests(&mut self) {
        self.parse_servo_test();
        self.parse_stepper_test();
        self.parse_value_error_test();
        self.parse_not_a_comand_error();
        self.parse_sepparator_error();
    }
}
