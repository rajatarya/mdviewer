// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::File;
use std::io::{Read, Write};

fn main() {
    if mdviewer_core::has_help_flag() {
        print!("{}", mdviewer_core::help_message());
        std::process::exit(0);
    }

    // Handle --open-url flag: macOS spawns a new process when double-clicking a file.
    // Extract file paths from --open-url args and write them to a temp file
    // so the main process can pick them up.
    let open_urls: Vec<String> = std::env::args()
        .filter(|a| a.starts_with("--open-url="))
        .map(|a| {
            let url = a.trim_start_matches("--open-url=");
            // Convert file:// URLs to local paths
            if let Some(path) = url.strip_prefix("file://") {
                path.to_string()
            } else {
                url.to_string()
            }
        })
        .collect();

    if !open_urls.is_empty() {
        // There's already a running instance — signal it via a temp file.
        let tmp_dir = std::env::temp_dir().join("mdviewer");
        std::fs::create_dir_all(&tmp_dir).ok();
        let signal_file = tmp_dir.join("open_urls.txt");
        if let Ok(mut f) = File::create(&signal_file) {
            for url in &open_urls {
                let _ = writeln!(f, "{}", url);
            }
        }
        std::process::exit(0);
    }

    // Single-instance guard: use a file lock so only one instance runs.
    let lock_path = std::env::temp_dir().join("mdviewer.lock");
    if let Ok(mut existing) = File::open(&lock_path) {
        // Try to read the PID of the existing instance
        let mut pid_str = String::new();
        if existing.read_to_string(&mut pid_str).is_ok() {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Check if that process is still alive
                if !is_process_alive(pid) {
                    // Stale lock — remove it and continue
                    let _ = std::fs::remove_file(&lock_path);
                } else {
                    // Another instance is running — signal it and exit
                    eprintln!("[mdviewer] Another instance is running, exiting.");
                    let tmp_dir = std::env::temp_dir().join("mdviewer");
                    std::fs::create_dir_all(&tmp_dir).ok();
                    let signal_file = tmp_dir.join("open_urls.txt");
                    if let Ok(mut f) = File::create(&signal_file) {
                        for arg in std::env::args().skip(1).filter(|a| !a.starts_with('-')) {
                            let _ = writeln!(f, "{}", arg);
                        }
                    }
                    std::process::exit(0);
                }
            }
        }
    }

    // Create our lock file with our PID
    if let Ok(mut f) = File::create(&lock_path) {
        let _ = writeln!(f, "{}", std::process::id());
    }

    mdviewer_core::run();

    // Clean up lock file on exit
    let _ = std::fs::remove_file(lock_path);
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        false
    }
}
