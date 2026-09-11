//! PTY process execution wrapper using portable-pty.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{bail, Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
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

    // Pump PTY master stdout to host stdout while buffering captured bytes
    let output_handle = thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut stdout = std::io::stdout();
        loop {
            match master_reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = &buffer[..n];
                    let _ = stdout.write_all(chunk);
                    let _ = stdout.flush();
                    if let Ok(mut captured) = captured_clone.lock() {
                        captured.extend_from_slice(chunk);
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Pump host stdin to PTY master until EOF or child exit
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

    let exit_status = child
        .wait()
        .context("Failed while waiting for child process")?;
    debug!(status = ?exit_status, "Child process exited");
    is_running.store(false, Ordering::SeqCst);

    // Dropping master closes the pseudo-console on ConPTY/PTY, terminating reader
    drop(pair.master);

    let _ = output_handle.join();
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
    fn test_run_pty_captures_stdout_from_command() {
        let pty_result = run_pty(&[
            "bash".to_string(),
            "-c".to_string(),
            "echo hola".to_string(),
        ])
        .expect("run_pty failed");
        let output_str = String::from_utf8_lossy(&pty_result.output);
        assert!(output_str.contains("hola"));
        assert_eq!(pty_result.exit_code, 0);
    }

    #[test]
    fn test_run_pty_propagates_child_exit_code() {
        let pty_result = run_pty(&["bash".to_string(), "-c".to_string(), "exit 42".to_string()])
            .expect("run_pty failed");
        assert_eq!(pty_result.exit_code, 42);
    }
}
