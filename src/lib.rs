pub mod parser;
pub mod datastream;
mod formats;
mod tokenizer;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        
        let a = "a";

        
        assert_eq!(result, 4);
    }
}
