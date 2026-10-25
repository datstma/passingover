//! `passingover` — where the planes above you are going.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "passingover",
    version,
    about = "Where the planes above you are going."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check that the CLI is wired to the core.
    Hello,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Hello => println!("{}", passingover_core::hello()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
