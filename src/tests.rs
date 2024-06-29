use core::fmt::Write;

use crate::protocol::{message::Message, parser::parse};

pub mod parsing_test;

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
    fn parse_data_test(&mut self) {
        let message = "ANGLE:90,";
        let res = parse(message);
        let expected = Message::ServoAngle(90.0);
        self.assert_eq(res, Ok(expected))
    }
    fn assert_eq<U: PartialEq>(&mut self, res: U, expected: U) {
        if res == expected {
            self.tx.write_str("PASSED").unwrap();
        } else {
            self.tx.write_str("FAILED").unwrap();
            panic!();
        }
    }

    pub fn run_tests(&mut self) {
        self.parse_data_test();
    }
}
