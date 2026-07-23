use thiserror::Error;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn release_url() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!(
        "https://github.com/globetrippy/project-white/releases/latest/download/pw-{os}-{arch}"
    )
}

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("failed to locate current executable: {0}")]
    CurrentExe(std::io::Error),
    #[error("failed to download update: {0}")]
    Download(reqwest::Error),
    #[error("downloaded file is not a valid {os} executable")]
    NotValid { os: String },
    #[error("failed to write update: {0}")]
    Io(std::io::Error),
}

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const MACHO_MAGICS: &[[u8; 4]] = &[
    [0xfe, 0xed, 0xfa, 0xce],
    [0xce, 0xfa, 0xed, 0xfe],
    [0xfe, 0xed, 0xfa, 0xcf],
    [0xcf, 0xfa, 0xed, 0xfe],
    [0xca, 0xfe, 0xba, 0xbe],
];
const PE_MAGIC: [u8; 2] = [b'M', b'Z']; // Windows PE executable signature

/// Download the latest `pw` binary from GitHub Releases and replace
/// the currently running executable in-place.
pub async fn update() -> Result<(), UpdateError> {
    let current_exe = std::env::current_exe().map_err(UpdateError::CurrentExe)?;
    let url = release_url();

    println!(
        "  Current version: {}",
        console::Style::new().bold().apply_to(VERSION)
    );
    println!(
        "  Downloading from: {}",
        console::Style::new().dim().apply_to(&url)
    );

    // download
    let client = reqwest::Client::builder()
        .user_agent(concat!("pw/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(UpdateError::Download)?;
    let resp = client.get(&url).send().await.map_err(UpdateError::Download)?;
    let status = resp.status();
    if !status.is_success() {
        eprintln!(
            "  error: server returned {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );
        std::process::exit(1);
    }
    let bytes = resp.bytes().await.map_err(UpdateError::Download)?;

    // verify
    validate_binary(&bytes)?;

    // write temp next to current exe
    let temp_path = current_exe.with_extension("new");
    tokio::fs::write(&temp_path, &bytes)
        .await
        .map_err(UpdateError::Io)?;

    #[cfg(unix)]
    {
        let meta = std::fs::metadata(&current_exe).map_err(UpdateError::Io)?;
        std::fs::set_permissions(&temp_path, meta.permissions()).map_err(UpdateError::Io)?;
    }

    // replace - on Windows, we need to handle the case where the exe is running
    #[cfg(windows)]
    {
        // On Windows, we can't overwrite a running executable.
        // Use MoveFileEx with MOVEFILE_DELAY_UNTIL_REBOOT or rename to .old and schedule replacement.
        // For simplicity, we'll copy to a .new file and instruct user to replace manually.
        use std::os::windows::fs::MetadataExt;
        // Copy permissions from current exe to new file
        if let Ok(meta) = std::fs::metadata(&current_exe) {
            let _ = std::fs::set_permissions(&temp_path, meta.permissions());
        }
    }

    // Try atomic rename (works on Unix, fails on Windows if file in use)
    match std::fs::rename(&temp_path, &current_exe) {
        Ok(()) => {}
        Err(e) if cfg!(windows) && e.kind() == std::io::ErrorKind::PermissionDenied => {
            // On Windows, the exe is locked while running.
            // Move current exe to .old, then move new to current name.
            let old_path = current_exe.with_extension("old");
            let _ = std::fs::remove_file(&old_path); // ignore error if doesn't exist
            std::fs::rename(&current_exe, &old_path).map_err(UpdateError::Io)?;
            std::fs::rename(&temp_path, &current_exe).map_err(UpdateError::Io)?;
            println!(
                "  {} Updated to latest version (old binary saved as .old)",
                console::Style::new().green().apply_to("✔")
            );
            println!("  Restart pw to use the new binary.");
            return Ok(());
        }
        Err(e) => return Err(UpdateError::Io(e)),
    }

    println!(
        "  {} Updated to latest version",
        console::Style::new().green().apply_to("✔")
    );
    println!("  Restart pw to use the new binary.");
    Ok(())
}

fn validate_binary(bytes: &[u8]) -> Result<(), UpdateError> {
    let header = bytes.get(..4).ok_or_else(|| UpdateError::NotValid {
        os: std::env::consts::OS.to_string(),
    })?;

    let ok = if cfg!(target_os = "linux") {
        header == ELF_MAGIC
    } else if cfg!(target_os = "macos") {
        // header is &[u8]; construct a [u8; 4] from the 4 bytes we verified exist
        let h = [header[0], header[1], header[2], header[3]];
        MACHO_MAGICS.contains(&h)
    } else if cfg!(target_os = "windows") {
        // Windows PE executables start with MZ (0x4D 0x5A)
        bytes.len() >= 2 && bytes[0] == PE_MAGIC[0] && bytes[1] == PE_MAGIC[1]
    } else {
        // Unknown OS - skip validation
        true
    };

    if !ok {
        return Err(UpdateError::NotValid {
            os: std::env::consts::OS.to_string(),
        });
    }
    Ok(())
}