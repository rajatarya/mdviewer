// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if mdviewer_core::has_help_flag() {
        print!("{}", mdviewer_core::help_message());
        std::process::exit(0);
    }

    mdviewer_core::run();
}
