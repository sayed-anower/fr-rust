use std::io::{self, Write};
use rand::Rng;

pub fn input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().expect("Input Failed!");
    let mut input_string = String::new();
    io::stdin().read_line(&mut input_string).expect("Failed to read line");
    input_string.trim_end().to_string()
}

pub fn generate_token(length: usize) -> String {
    let num_bytes = length / 2;
    let mut bytes = vec![0u8; num_bytes];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}
