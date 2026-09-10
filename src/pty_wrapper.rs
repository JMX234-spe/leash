//! PTY process execution wrapper using portable-pty.

use anyhow::{bail, Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tracing::debug;

/// Result of executing a command inside a pseudo-terminal.
#[derive(Debug, Clone)]
pub struct PtyResult {
    pub exit_code: i32,
    pub output: Vec<u8>,
}

/// Spawns a command wrapped in a pseudo-terminal, forwarding stdin/stdout/stderr
/// and optionally capturing all emitted output.
pub fn run_pty(command: &[String]) -> Result<PtyResult> {
    if command.is_empty() {
        bail!("No command specified to run in PTY wrapper");
    }

    let pty_system = native_pty_system();
    let size = PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(size)
        .context("Failed to open pseudo-terminal pair")?;

    let mut cmd_builder = CommandBuilder::new(&command[0]);
    for arg in &command[1..] {
        cmd_builder.arg(arg);
    }

    debug!(command = ?command, "Spawning child process in PTY");

    // Spawn child process in slave PTY
    let mut child = pair
        .slave
        .spawn_command(cmd_builder)
        .with_context(|| format!("Failed to spawn command '{}' in PTY", command[0]))?;

    // Drop slave handle in parent so EOF propagates when child terminates
    drop(pair.slave);

    let mut master_reader = pair
        .master
        .try_clone_reader()
        .context("Failed to clone PTY master reader")?;
    let mut master_writer = pair
        .master
        .take_writer()
        .context("Failed to take PTY master writer")?;

    let captured_output = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = Arc::clone(&captured_output);
    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_writer = Arc::clone(&is_running);

    // Thread 1: Read from PTY master and forward to host stdout, while accumulating output
    let output_handle = thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut stdout = std::io::stdout();
        loop {
            match master_reader.read(&mut buffer) {
                Ok(0) => break, // EOF reached
                Ok(n) => {
                    let chunk = &buffer[..n];
                    let _ = stdout.write_all(chunk);
                    let _ = stdout.flush();
                    if let Ok(mut captured) = captured_clone.lock() {
                        captured.extend_from_slice(chunk);
                    }
                }
                Err(_) => break, // Master closed or broken pipe
            }
        }
    });

    // Thread 2: Read from host stdin and forward to PTY master
    let input_handle = thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buffer = [0u8; 1024];
        while is_running_writer.load(Ordering::Relaxed) {
            match stdin.read(&mut buffer) {
                Ok(0) => {
                    // Stdin reached EOF. Keep writer alive until child finishes
                    // to prevent ConPTY from interpreting premature pipe closure as console hangup.
                    while is_running_writer.load(Ordering::Relaxed) {
                        thread::sleep(std::time::Duration::from_millis(20));
                    }
                    break;
                }
                Ok(n) => {
                    if master_writer.write_all(&buffer[..n]).is_err() {
                        break;
                    }
                    let _ = master_writer.flush();
                }
                Err(_) => break,
            }
        }
    });

    // Wait for the child process to exit
    let exit_status = child
        .wait()
        .context("Failed while waiting for child process")?;
    debug!(status = ?exit_status, "Child process exited");
    is_running.store(false, Ordering::SeqCst);

    // Dropping master closes the pseudo-console on ConPTY/PTY, terminating reader
    drop(pair.master);

    // Wait for output thread to finish consuming remaining bytes
    let _ = output_handle.join();

    // Input handle will either finish on EOF or terminate with process
    drop(input_handle);

    let exit_code = if exit_status.success() {
        0
    } else {
        exit_status.exit_code() as i32
    };

    let output = captured_output
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();

    Ok(PtyResult { exit_code, output })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_hola() {
        let res = run_pty(&[
            "bash".to_string(),
            "-c".to_string(),
            "echo hola".to_string(),
        ])
        .expect("run_pty failed");
        let output_str = String::from_utf8_lossy(&res.output);
        assert!(output_str.contains("hola"));
        assert_eq!(res.exit_code, 0);
    }

    #[test]
    fn test_non_zero_exit_code() {
        let res = run_pty(&["bash".to_string(), "-c".to_string(), "exit 42".to_string()])
            .expect("run_pty failed");
        assert_eq!(res.exit_code, 42);
    }
}
