use yule_log::{ULogMessages, ULogData};

#[derive(ULogData)]
pub struct A { value: u64 }

#[derive(ULogMessages)]
pub enum LoggedMessages {
    A(A, A),
}

fn main() {}
