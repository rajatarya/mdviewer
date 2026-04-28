// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;

fn is_md_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".txt")
}

fn main() {
    let cli_paths: Vec<String> = env::args().skip(1).filter(|p| is_md_file(p)).collect();

    if !cli_paths.is_empty() {
        eprintln!("[mdviewer] Opening: {:?}", cli_paths);
    }

    mdviewer_core::run(cli_paths);
}
