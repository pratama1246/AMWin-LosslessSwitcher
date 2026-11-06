//! RateAlign (Windows x64)
//! Developed by Pratama Putra (https://github.com/pratama1246/AMWin-LosslessSwitcher)
//!
//! Inspired by Vincent Neo (LosslessSwitcher for macOS) and Jordan Gibson (WindowsLosslessSwitcher).
//! Licensed under GNU General Public License v3.0.

#![windows_subsystem = "windows"]

mod audio;
mod apple_music;
mod coordinator;
mod config;
mod ui;

fn main() {
    ui::run_app();
}
