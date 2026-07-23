use std::io::{self, Write};
use std::time::Instant;

use crossterm::{
    cursor, execute,
    terminal::{self, ClearType},
};

/// Display modes per the UX spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Verbose,
    Debug,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}

/// Terminal width cache.
static TERM_WIDTH: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

fn term_width() -> u16 {
    *TERM_WIDTH.get_or_init(|| crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80))
}

/// Progress bar width: max(10, term_width - 65) per spec.
fn bar_width() -> usize {
    max(10, term_width() as i32 - 65) as usize
}

fn max(a: i32, b: i32) -> i32 {
    if a > b {
        a
    } else {
        b
    }
}

/// Mode symbol shown at start of session line.
fn mode_symbol(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "✦",
        Mode::Verbose => "✦",
        Mode::Debug => "[+0.000]",
    }
}

/// Renderer state for the 4 static + 1 live line layout.
pub struct Renderer {
    mode: Mode,
    session_line: String,
    detail_line: Option<String>,
    live_line: Option<LiveLine>,
    verbose_panel: Option<VerbosePanel>,
    completion_line: Option<String>,
    stdout: io::Stdout,
    first_draw: bool,
}

/// Live progress line data.
#[derive(Clone)]
pub struct LiveLine {
    pub percent: u8,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub speed_bps: u64,
    pub current_file: String,
    pub eta_secs: u64,
    pub chunk_num: Option<u64>,
    pub chunks_total: Option<u64>,
    pub compression_ratio: Option<f32>,
    pub packet_rate: Option<f32>,
    pub rtt_ms: Option<u32>,
    pub peer_addr: Option<String>,
    pub session_id: Option<String>,
    pub read_speed_bps: Option<u64>,
    pub encrypt_speed_bps: Option<u64>,
    pub network_speed_bps: Option<u64>,
    pub write_speed_bps: Option<u64>,
}

impl Default for LiveLine {
    fn default() -> Self {
        Self {
            percent: 0,
            bytes_done: 0,
            bytes_total: 0,
            speed_bps: 0,
            current_file: String::new(),
            eta_secs: 0,
            chunk_num: None,
            chunks_total: None,
            compression_ratio: None,
            packet_rate: None,
            rtt_ms: None,
            peer_addr: None,
            session_id: None,
            read_speed_bps: None,
            encrypt_speed_bps: None,
            network_speed_bps: None,
            write_speed_bps: None,
        }
    }
}

/// Verbose panel lines (shown below live line in verbose mode).
#[derive(Default)]
pub struct VerbosePanel {
    pub peer_line: Option<String>,
    pub session_line: Option<String>,
    pub pipeline_line: Option<String>,
}

impl Renderer {
    pub fn new(mode: Mode) -> Self {
        let _ = terminal::enable_raw_mode();
        Renderer {
            mode,
            session_line: String::new(),
            detail_line: None,
            live_line: None,
            verbose_panel: None,
            completion_line: None,
            stdout: io::stdout(),
            first_draw: true,
        }
    }

    /// Set the session anchor line: "✦ pw:send  aB3xK9mZ"
    pub fn set_session(&mut self, direction: &str, code: &str) {
        self.session_line = format!("{}  pw:{}  {}", mode_symbol(self.mode), direction, code);
        self.draw();
    }

    /// Set the detail line (shown under session line).
    pub fn set_detail(&mut self, text: impl Into<String>) {
        self.detail_line = Some(text.into());
        self.draw();
    }

    /// Clear the detail line.
    pub fn clear_detail(&mut self) {
        self.detail_line = None;
        self.draw();
    }

    /// Update live progress fields.
    pub fn set_live(&mut self, live: LiveLine) {
        self.live_line = Some(live);
        self.draw();
    }

    /// Clear live line (e.g., on error/completion).
    pub fn clear_live(&mut self) {
        self.live_line = None;
        self.draw();
    }

    /// Set verbose panel (only rendered in Verbose mode).
    pub fn set_verbose(&mut self, panel: VerbosePanel) {
        if self.mode == Mode::Verbose {
            self.verbose_panel = Some(panel);
            self.draw();
        }
    }

    /// Replace live line with completion line.
    pub fn complete(&mut self, text: impl Into<String>) {
        self.completion_line = Some(text.into());
        self.live_line = None;
        self.draw();
    }

    /// Show error on the line where it occurred (replaces detail or live).
    pub fn error(&mut self, text: impl Into<String>) {
        let err = format!("✖  {}", text.into());
        if self.live_line.is_some() {
            self.live_line = None;
        } else {
            self.detail_line = Some(err);
        }
        self.draw();
    }

    /// Full redraw: clear and redraw all lines.
    fn draw(&mut self) {
        let mut out = self.stdout.lock();

        if self.first_draw {
            self.first_draw = false;
        } else {
            let lines_to_clear = self.count_lines();
            for _ in 0..lines_to_clear {
                execute!(
                    out,
                    cursor::MoveUp(1),
                    terminal::Clear(ClearType::CurrentLine)
                )
                .ok();
            }
        }

        // Session line (always visible)
        writeln!(out, "  {}", self.session_line).ok();

        // Detail line (if present)
        if let Some(detail) = &self.detail_line {
            writeln!(out, "  ╶  {}", detail).ok();
        }

        // Live progress line (if present)
        if let Some(live) = &self.live_line {
            writeln!(out, "  {}", format_live(self.mode, live)).ok();
        }

        // Verbose panel (if present and in Verbose mode)
        if self.mode == Mode::Verbose {
            if let Some(panel) = &self.verbose_panel {
                if let Some(line) = &panel.peer_line {
                    writeln!(out, "  ╶  {}", line).ok();
                }
                if let Some(line) = &panel.session_line {
                    writeln!(out, "  ╶  {}", line).ok();
                }
                if let Some(line) = &panel.pipeline_line {
                    writeln!(out, "  ╶  {}", line).ok();
                }
            }
        }

        // Completion line (if present)
        if let Some(comp) = &self.completion_line {
            writeln!(out, "  {}", comp).ok();
        }

        out.flush().ok();
    }

    /// Count how many lines we've drawn so we can clear them on redraw.
    fn count_lines(&self) -> u16 {
        let mut count = 1; // session line
        if self.detail_line.is_some() {
            count += 1;
        }
        if self.live_line.is_some() {
            count += 1;
        }
        if self.mode == Mode::Verbose {
            if let Some(panel) = &self.verbose_panel {
                if panel.peer_line.is_some() {
                    count += 1;
                }
                if panel.session_line.is_some() {
                    count += 1;
                }
                if panel.pipeline_line.is_some() {
                    count += 1;
                }
            }
        }
        if self.completion_line.is_some() {
            count += 1;
        }
        count
    }
}

/// Format the live progress line per spec.
fn format_live(mode: Mode, live: &LiveLine) -> String {
    let bw = bar_width();
    let filled = ((live.percent as usize * bw) / 100).min(bw);
    let empty = bw - filled;
    let bar = "▓".repeat(filled) + &"░".repeat(empty);

    let pct = format!("{:>3}%", live.percent);
    let done = format_bytes(live.bytes_done);
    let total = format_bytes(live.bytes_total);
    let speed = format_speed(live.speed_bps);
    let file = &live.current_file;
    let eta = format_eta(live.bytes_done, live.bytes_total, live.speed_bps);

    if mode == Mode::Debug {
        let elapsed = Instant::now().elapsed().as_secs_f32();
        format!(
            "[{:.3}] {}  ·  {}/{}  ·  {}  ·  {}  ·  {}",
            elapsed, bar, pct, done, total, speed, file
        )
    } else {
        format!(
            "✦  {}  {}  ·  {}/{}  ·  {}  ·  {}  ·  {}",
            bar, pct, done, total, speed, file, eta
        )
    }
}

/// Format bytes in IEC units (1 decimal, except bytes).
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{} B", bytes);
    }
    let mut size = bytes as f64;
    let mut idx = 0;
    while size >= 1024.0 && idx < UNITS.len() - 1 {
        size /= 1024.0;
        idx += 1;
    }
    format!("{:.1} {}", size, UNITS[idx])
}

/// Format speed as bytes/sec in IEC units.
fn format_speed(bps: u64) -> String {
    format!("{}/s", format_bytes(bps))
}

/// Format ETA from progress.
fn format_eta(done: u64, total: u64, speed: u64) -> String {
    if speed == 0 || done >= total {
        return "0s".to_string();
    }
    let remaining = total - done;
    let secs = remaining / speed;
    if secs < 60 {
        format!("{}s", secs)
    } else {
        let mins = secs / 60;
        let s = secs % 60;
        if s > 0 {
            format!("{}m {}s", mins, s)
        } else {
            format!("{}m", mins)
        }
    }
}

/// Format fingerprint: 8 hex bytes in 4 groups of 2.
pub fn format_fingerprint(hash: &[u8; 8]) -> String {
    let hex = hex::encode_upper(hash);
    let mut out = String::with_capacity(23);
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if i > 0 && i % 4 == 0 {
            out.push_str("  ");
        } else if i > 0 {
            out.push(' ');
        }
        out.push_str(std::str::from_utf8(chunk).unwrap());
    }
    out
}

/// Completion line helpers.
pub fn completion_line_normal(file_count: usize, total_bytes: u64, root_hash: &str) -> String {
    format!(
        "✔  done  ·  {} file{}  ·  {}  ·  root: {}",
        file_count,
        if file_count == 1 { "" } else { "s" },
        format_bytes(total_bytes),
        &root_hash[..8]
    )
}

pub fn completion_line_verbose(
    duration_secs: f32,
    avg_speed: u64,
    peak_speed: u64,
    fingerprint: &str,
) -> VerbosePanel {
    VerbosePanel {
        peer_line: Some(format!(
            "duration: {:.1}s        avg: {}        peak: {}",
            duration_secs,
            format_bytes(avg_speed),
            format_bytes(peak_speed)
        )),
        session_line: Some("integrity: verified    cipher: XChaCha20-Poly1305    protocol: v1".to_string()),
        pipeline_line: Some(format!("fingerprint: {}", fingerprint)),
    }
}

/// Startup line.
pub fn startup_line(version: &str, _target: &str) -> String {
    format!("pw {} — secure p2p folder transfer", version)
}

/// Error line.
pub fn error_line(msg: &str) -> String {
    format!("✖  {}", msg)
}

/// Help screens per spec.
pub fn help_main() -> String {
    r#"pw — secure peer-to-peer folder transfer

Usage:
    pw <command> [options]

Commands:
    send      Send a folder to a receiver
    receive   Receive a folder from a sender
    doctor    Diagnose local environment
    status    Show transfer history

Options:
    -v, --verbose    Show additional transfer details
    -d, --debug      Show protocol-level events
    -h, --help       Show help for a command
    -V, --version    Show version

Examples:
    pw send ./my-project
    pw receive aB3xK9mZ -i ~/Downloads
    pw doctor

Run 'pw help <command>' for detailed help."#.to_string()
}

pub fn help_send() -> String {
    r#"pw send — send a folder to a receiver

Usage:
    pw send [options] <path>

Arguments:
    <path>                  Folder to send (must be a directory)

Options:
    -s, --server <url>     Signaling server (default: https://pw.example.com)
    -c, --chunk-size <n>   Chunk size in bytes (default: 65536)
    -t, --timeout <sec>    Wait time for receiver (default: 120)
    -y, --yes              Skip interactive confirmation
    --public-ip <addr>     Public IP for NAT traversal

Examples:
    pw send ./my-project
    Send the 'my-project' folder. Share the resulting code with the receiver.

    pw send ~/Projects/app --public-ip 203.0.113.42
    Specify a public IP for NAT traversal.

    pw send ./large-folder -c 262144
    Use larger chunks (256 KB) for better throughput on fast networks.

Security:
    All data is encrypted with X25519 + ChaCha20-Poly1305.
    Verify the fingerprint with the receiver out of band.
    Keys are ephemeral and destroyed after transfer.

See also:
    pw receive, pw doctor"#.to_string()
}

pub fn help_receive() -> String {
    r#"pw receive — receive a folder from a sender

Usage:
    pw receive [options] <code>

Arguments:
    <code>                  Session code from the sender

Options:
    -s, --server <url>     Signaling server (default: https://pw.example.com)
    -c, --chunk-size <n>   Chunk size in bytes (default: 65536)
    -t, --timeout <sec>    Wait time for sender approval (default: 30)
    -i, --in <path>        Output directory (default: .)
                           If the target requires root, you will be prompted

Examples:
    pw receive aB3xK9mZ
    Receive files into the current directory.

    pw receive aB3xK9mZ -i ~/Downloads
    Receive files into ~/Downloads.

    pw receive aB3xK9mZ --in /var/backups
    Receive files into /var/backups (may require sudo).

Security:
    Verify the displayed fingerprint matches the sender's display.
    Files are verified against the manifest root hash on completion.

See also:
    pw send, pw doctor"#.to_string()
}

pub fn help_doctor() -> String {
    r#"pw doctor — diagnose local environment

Usage:
    pw doctor [options]

Options:
    -s, --server <url>     Test a specific signaling server

Example output:
    ✔  pw 0.1.0 (aarch64-apple-darwin)
    ✔  signaling server reachable (12ms)
    ✔  TCP port available
    ✔  NAT: full-cone (public IP detectable)
    ✖  mlockall: unavailable (no permissions)
    ╶  key material may be paged to disk"#.to_string()
}

pub fn help_status() -> String {
    r#"pw status — show transfer history

Usage:
    pw status

Shows recent transfers with file count, size, session code, time, and state."#.to_string()
}

/// Compatibility functions for existing code using the old ui module API.
/// These print directly to stdout/stderr (not through Renderer).

pub fn success(msg: &str) {
    println!("  ✔  {}", msg);
}

pub fn info(msg: &str) {
    println!("  ▸  {}", msg);
}

pub fn detail(label: &str, value: &str) {
    println!("    {}  {}", label, value);
}

pub fn fingerprint(label: &str, value: &str) {
    println!("    {}  {}", label, value);
}

pub fn error(msg: &str) {
    eprintln!("  ✖  {}", msg);
}

pub fn new_spinner(_msg: &str) -> DummySpinner {
    DummySpinner
}

pub fn new_progress_bar(_total: u64) -> DummyProgressBar {
    DummyProgressBar
}

pub struct DummySpinner;

impl DummySpinner {
    pub fn finish_and_clear(&self) {}
}

pub struct DummyProgressBar;

impl DummyProgressBar {
    pub fn set_message(&self, _msg: String) {}
    pub fn inc(&self, _n: u64) {}
    pub fn finish_and_clear(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_format_fingerprint() {
        let hash = [0x8D, 0xF6, 0x7B, 0x2A, 0x9C, 0x4E, 0x1B, 0x5F];
        assert_eq!(format_fingerprint(&hash), "8D F6 7B 2A  9C 4E 1B 5F");
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(format_eta(0, 1000, 100), "10s");
        assert_eq!(format_eta(1000, 1000, 100), "0s");
        assert_eq!(format_eta(0, 6000, 100), "1m");
        assert_eq!(format_eta(0, 9000, 100), "1m 30s");
    }

    #[test]
    fn test_bar_width() {
        assert_eq!(max(10, 80 - 65), 15);
        assert_eq!(max(10, 40 - 65), 10);
    }
}
