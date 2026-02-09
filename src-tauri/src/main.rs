// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use anyhow::anyhow;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    unsafe {
        if env::var("WAYLAND_DISPLAY").is_ok() {
            env::set_var("GDK_BACKEND", "wayland");
        }
    }

    let args = diary_tauri_lib::cli::Args::parse();
    diary_tauri_lib::run(args);
    Ok(())
}
