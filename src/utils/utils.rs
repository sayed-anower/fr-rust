use std::io::{self, Write};
use rand::RngCore;

fn input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().expect("Input Failed!");
    input_string.trim_end().to_string()
}
fn generate_key(length: usize) -> String {
    let num_bytes = length / 2;
    let mut bytes = vec![0u8; num_bytes];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}
