//! VaultPass Native Messaging Host
//!
//! Bridges Chrome/Firefox native messaging (stdin/stdout, 4-byte LE length-prefix)
//! ↔ VaultPass desktop Tauri app via named pipe / Unix socket.
//!
//! Protocol both directions: [u32 LE length][JSON bytes]

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

#[cfg(windows)]
fn open_pipe() -> io::Result<std::fs::File> {
    use std::fs::OpenOptions;
    // Named pipes on Windows are accessible via the filesystem path.
    // Retry a few times in case the Tauri app is still starting.
    let mut last_err = io::Error::new(io::ErrorKind::ConnectionRefused, "pipe not ready");
    for _ in 0..8 {
        match OpenOptions::new().read(true).write(true).open(PIPE_PATH) {
            Ok(f) => return Ok(f),
            Err(e) => {
                last_err = e;
                std::thread::sleep(std::time::Duration::from_millis(500));
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
    use std::os::unix::net::UnixStream;
    for _ in 0..8 {
        match UnixStream::connect(PIPE_PATH) {
            Ok(s) => return Ok(s.into()),
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(500)),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::ConnectionRefused,
        format!("Cannot connect to {PIPE_PATH} — is VaultPass desktop running?"),
    ))
}
