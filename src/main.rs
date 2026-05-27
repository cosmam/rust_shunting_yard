use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn Error>> {
    let variables: HashMap<String, shunting_yard::Value> = HashMap::new();

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

        let _ = shunting_yard::evaluate(&input, &variables)
            .map_err(|e| eprintln!("Detailed log: {:?}", e));
    }

    Ok(())
}
