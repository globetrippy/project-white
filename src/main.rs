use clap::Parser;
use project_white::cli::Cli;

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        project_white::cli::Command::Send { .. } => {
            eprintln!("pw send: not yet implemented (coming in Milestone 3)");
        }
        project_white::cli::Command::Receive { .. } => {
            eprintln!("pw receive: not yet implemented (coming in Milestone 3)");
        }
    }
}
