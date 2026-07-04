use clap::Parser;
use project_white::cli::Cli;

fn main() {
    let cli = Cli::parse();

    let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");

    match &cli.command {
        project_white::cli::Command::Send {
            path,
            server,
            chunk_size,
            timeout,
            yes,
            public_ip,
        } => {
            if !path.exists() {
                eprintln!("error: path does not exist: {}", path.display());
                std::process::exit(1);
            }
            if !path.is_dir() {
                eprintln!("error: path is not a directory: {}", path.display());
                std::process::exit(1);
            }

            if let Err(e) = rt.block_on(project_white::transfer::sender::send_folder(
                server,
                path,
                *chunk_size,
                *timeout,
                public_ip.clone(),
                *yes,
            )) {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
        project_white::cli::Command::Receive {
            code,
            server,
            chunk_size,
            timeout,
            output,
        } => {
            if let Err(e) = rt.block_on(project_white::transfer::receiver::receive_folder(
                server,
                code,
                *chunk_size,
                *timeout,
                output,
            )) {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
