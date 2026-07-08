use std::time::Duration;

use console::style;
use indicatif::{ProgressBar, ProgressStyle};

pub fn info(msg: &str) {
    println!("  {} {}", style("▸").cyan(), msg);
}

pub fn success(msg: &str) {
    println!("  {} {}", style("✔").green().bold(), style(msg).green());
}

pub fn detail(label: &str, value: &str) {
    println!("    {} {}", style(label).dim(), style(value).bold());
}

pub fn fingerprint(label: &str, value: &str) {
    println!(
        "    {} {}",
        style(label).dim(),
        style(value).yellow().bold()
    );
}

pub fn error(msg: &str) {
    eprintln!("  {} {}", style("✗").red().bold(), style(msg).red());
}

pub fn new_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["◐", "◓", "◑", "◒"])
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

pub fn new_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:32.cyan/blue}] {bytes}/{total_bytes}  {msg}")
            .expect("valid template")
            .progress_chars("▓▓░"),
    );
    pb
}

pub fn section(msg: &str) {
    println!();
    println!("  {}", style(msg).bold());
}
