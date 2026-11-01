#![windows_subsystem = "windows"]

mod audio;
mod apple_music;
mod coordinator;
mod config;
mod ui;

fn main() {
    ui::run_app();
}
