use yule_log::ULogData;

#[derive(ULogData)]
pub struct GenericStruct<T> {
    value: T,
}

fn main() {}
