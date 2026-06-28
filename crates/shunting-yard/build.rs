#![deny(
    clippy::expect_used,
    clippy::todo,
    clippy::unimplemented,
    clippy::unwrap_used
)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    lalrpop::process_root()?;
    Ok(())
}
