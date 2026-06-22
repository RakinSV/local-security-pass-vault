//! VaultPass Native Messaging Host
//!
//! Bridges Chrome/Firefox native messaging (stdin/stdout, 4-byte LE length-prefix)
//! ↔ VaultPass desktop Tauri app via named pipe / Unix socket.
//!
//! Protocol both directions: [u32 LE length][JSON bytes]

// Hide the console window on Windows — native messaging hosts must not create
// a visible console when launched by Chrome/Firefox.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::io::{self, Read, Write};

#[cfg(windows)]
const PIPE_PATH: &str = r"\\.\pipe\vaultpass";

#[cfg(not(windows))]
const PIPE_PATH: &str = "/tmp/vaultpass.sock";

fn main() {
    if let Err(e) = run() {
        // Write error as a native message so the extension can surface it
        let msg = format!("{{\"error\":\"{e}\"}}");
        let _ = write_msg(&mut io::stdout().lock(), msg.as_bytes());
    }
}

fn run() -> io::Result<()> {
    let mut pipe = open_pipe()?;

    loop {
        // Read one native message from Chrome (stdin)
        let msg = read_msg(&mut io::stdin().lock())?;

        // Forward to Tauri pipe server
        write_msg(&mut pipe, &msg)?;
        pipe.flush()?;

        // Read response from Tauri pipe server
        let resp = read_msg(&mut pipe)?;

        // Send response back to Chrome (stdout)
        write_msg(&mut io::stdout().lock(), &resp)?;
        io::stdout().flush()?;
    }
}

/// Read a length-prefixed message: [u32 LE][bytes]
fn read_msg<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message size out of range"));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

/// Write a length-prefixed message: [u32 LE][bytes]
fn write_msg<W: Write>(w: &mut W, msg: &[u8]) -> io::Result<()> {
    w.write_all(&(msg.len() as u32).to_le_bytes())?;
    w.write_all(msg)
}

// Retry parameters: 20 × 500 ms = 10 seconds total.
// Extended from the previous 8 × 500 ms (4 s) so that opening the extension
// immediately after launching the desktop app succeeds even on slow machines.
const CONNECT_RETRIES: u32 = 20;
const CONNECT_SLEEP_MS: u64 = 500;

#[cfg(windows)]
fn open_pipe() -> io::Result<std::fs::File> {
    use std::fs::OpenOptions;
    let mut last_err = io::Error::new(io::ErrorKind::ConnectionRefused, "pipe not ready");
    for _ in 0..CONNECT_RETRIES {
        match OpenOptions::new().read(true).write(true).open(PIPE_PATH) {
            Ok(f) => return Ok(f),
            Err(e) => {
                last_err = e;
                std::thread::sleep(std::time::Duration::from_millis(CONNECT_SLEEP_MS));
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::ConnectionRefused,
        format!("Cannot connect to {PIPE_PATH}: {last_err} — is VaultPass desktop running?"),
    ))
}

#[cfg(not(windows))]
fn open_pipe() -> io::Result<std::fs::File> {
    use std::os::unix::io::{FromRawFd, IntoRawFd};
    use std::os::unix::net::UnixStream;
    let mut last_err = io::Error::new(io::ErrorKind::ConnectionRefused, "socket not ready");
    for _ in 0..CONNECT_RETRIES {
        match UnixStream::connect(PIPE_PATH) {
            Ok(s) => {
                // Convert UnixStream → File via raw fd (same kernel fd, different Rust wrapper)
                // Safety: into_raw_fd() transfers ownership; fd remains valid.
                return Ok(unsafe { std::fs::File::from_raw_fd(s.into_raw_fd()) });
            }
            Err(e) => {
                last_err = e;
                std::thread::sleep(std::time::Duration::from_millis(CONNECT_SLEEP_MS));
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::ConnectionRefused,
        format!("Cannot connect to {PIPE_PATH}: {last_err} — is VaultPass desktop running?"),
    ))
}
