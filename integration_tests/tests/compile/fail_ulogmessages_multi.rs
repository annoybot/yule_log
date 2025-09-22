use yule_log::{ULogMessages, ULogData};

#[derive(ULogData)]
pub struct A { value: u64 }
#[derive(ULogData)]
pub struct B { value: u64 }

#[derive(ULogMessages)]
pub enum LoggedMessages {
    #[yule_log(forward_other)]
    A(A),
    #[yule_log(forward_other)]
    B(B),
}

fn main() {}
