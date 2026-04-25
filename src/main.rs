use std::error::{Error};
use std::io::{self, Write};
use shunting_yard;

fn main() -> Result<(), Box<dyn Error>> {
    loop {
        let mut input = String::new();

        print!("Enter something: ");
        // Flush stdout so the prompt appears before reading
        io::stdout().flush().expect("Failed to flush");

        // Read the line into the input string
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        // Break the loop if the input is "exit" (trimming newline)
        if input.trim() == "exit" {
            break;
        }

        let _ = shunting_yard::evaluate(&input).map_err(|e| {
            eprintln!("Detailed log: {:?}", e);
            e // Return the original error
        })?;
    }

    Ok(())
}