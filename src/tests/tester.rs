use core::fmt::Write;
use function_name::named;

use crate::protocol::{
    message::Message,
    parser::{ParsingError, parse},
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
    #[named]
    fn parse_servo_test(&mut self) {
        let message = "ANGLE:90,";
        let res = parse(message);
        self.assert_eq(res, Ok(Message::ServoAngle(90.0)));

        writeln!(&mut self.tx, "{}: PASSED", function_name!()).unwrap();
    }

    #[named]
    fn parse_stepper_test(&mut self) {
        let message = "STEPS:25,";
        let res = parse(message);
        self.assert_eq(res, Ok(Message::StepperMotorRunSteps(25)));

        let message = "STOP:,";
        let res = parse(message);
        self.assert_eq(res, Ok(Message::StepperStop));

        writeln!(&mut self.tx, "{}: PASSED", function_name!()).unwrap();
    }

    #[named]
    fn parse_value_error_test(&mut self) {
        let message = "STEPS:,";
        let res = parse(message);
        self.assert_eq(res, Err(ParsingError::ValueCanNotBeParsed));
        writeln!(&mut self.tx, "{}: PASSED", function_name!()).unwrap();
    }
    #[named]
    fn parse_not_a_comand_error_test(&mut self) {
        let message = "STO:,";
        let res = parse(message);
        self.assert_eq(res, Err(ParsingError::NotAComand));

        writeln!(&mut self.tx, "{}: PASSED", function_name!()).unwrap();
    }

    #[named]
    fn parse_sepparator_error_test(&mut self) {
        let message = "STEPS4,";
        let res = parse(message);
        self.assert_eq(res, Err(ParsingError::NoSepparator));

        let message = "STEPS:4";
        let res = parse(message);
        self.assert_eq(res, Err(ParsingError::NoSepparator));

        writeln!(&mut self.tx, "{}: PASSED", function_name!()).unwrap();
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
        self.parse_not_a_comand_error_test();
        self.parse_sepparator_error_test();
    }
}
