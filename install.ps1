# Project-White — single-command installer for Windows
# Usage: iwr -Uri https://pw-server-gna4.onrender.com/install.ps1 | iex

$Server = if ($env:PW_SERVER) { $env:PW_SERVER } else { "https://pw-server-gna4.onrender.com" }

# ─── Platform Detection ──────────────────────────────────────────
$Arch = switch ([Environment]::Is64BitOperatingSystem) {
    $true  { "x86_64" }
    $false { "i686" }
}

$Binary = "pw-windows-$Arch"
$Url = "$Server/download/$Binary"
$DestDir = "$env:LOCALAPPDATA\pw"
$DestPath = "$DestDir\pw.exe"

# ─── Download ─────────────────────────────────────────────────────
Write-Host "  · Downloading Project-White for Windows/$Arch..."
Write-Host "  · Server: $Server"

try {
    $null = New-Item -ItemType Directory -Force -Path "$env:TEMP\pw" -ErrorAction Stop
    $TempFile = "$env:TEMP\pw\pw.exe"
    
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $Url -OutFile $TempFile -UseBasicParsing -ErrorAction Stop
    $ProgressPreference = 'Continue'
} catch {
    Write-Host ""
    Write-Host "  ✗ Binary not available for Windows/$Arch on this server yet."
    Write-Host "    Available platforms: linux-x86_64 (testing phase)"
    Write-Host ""
    Write-Host "    To build from source install Rust from https://rustup.rs then:"
    Write-Host "      cargo install project-white"
    exit 1
}

# ─── Install ─────────────────────────────────────────────────────
$null = New-Item -ItemType Directory -Force -Path $DestDir -ErrorAction Stop
Move-Item -Force -Path $TempFile -Destination $DestPath

# Add to PATH if not already present
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$DestDir*") {
    $NewPath = "$DestDir;$UserPath"
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    Write-Host "  · Added $DestDir to PATH (reopen terminal to use)"
}

Write-Host "  ✓ Installed to $DestPath"
Write-Host ""
Write-Host "  ✓ Project-White ready. Run: pw --help"
