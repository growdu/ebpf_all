use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "uof")]
#[command(about = "CLI for Universal Observability Framework")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) | None => {
            println!("uof-cli {}", env!("CARGO_PKG_VERSION"));
        }
    }
}

