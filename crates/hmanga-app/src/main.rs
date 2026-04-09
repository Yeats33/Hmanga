#![cfg_attr(
    all(feature = "bundle", target_os = "windows"),
    windows_subsystem = "windows"
)]

use dioxus::prelude::*;

mod app;
mod service;
mod state;

use crate::app::App;

fn main() {
    launch(App);
}
