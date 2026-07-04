use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Secure peer-to-peer folder transfer.
///
/// Send a folder directly to another machine with end-to-end encryption.
/// No accounts, no cloud, no configuration.
#[derive(Parser, Debug)]
#[command(name = "pw", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose logging.
    #[arg(short, long, global = true, default_value_t = false)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Send a folder to a receiver.
    Send {
        /// Path to the folder to send.
        path: PathBuf,

        /// Signaling server URL.
        #[arg(long, default_value = "https://pw.example.com", env = "PW_SERVER")]
        server: String,

        /// File chunk size in bytes.
        #[arg(long, default_value_t = 65536, env = "PW_CHUNK_SIZE")]
        chunk_size: usize,

        /// Idle timeout in seconds.
        #[arg(long, default_value_t = 120, env = "PW_TIMEOUT")]
        timeout: u64,

        /// Skip interactive confirmation when receiver connects.
        #[arg(long, default_value_t = false)]
        yes: bool,

        /// Public IP address for the receiver to connect to.
        /// Required if sender is behind NAT.
        #[arg(long, env = "PW_PUBLIC_IP")]
        public_ip: Option<String>,
    },

    /// Receive a folder from a sender.
    Receive {
        /// Session code provided by the sender.
        code: String,

        /// Signaling server URL.
        #[arg(long, default_value = "https://pw.example.com", env = "PW_SERVER")]
        server: String,

        /// File chunk size in bytes.
        #[arg(long, default_value_t = 65536, env = "PW_CHUNK_SIZE")]
        chunk_size: usize,

        /// Idle timeout in seconds.
        #[arg(long, default_value_t = 30, env = "PW_TIMEOUT")]
        timeout: u64,

        /// Output directory for received files.
        #[arg(long, default_value = ".", env = "PW_OUTPUT")]
        output: PathBuf,
    },
}
