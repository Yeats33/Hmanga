use dioxus::prelude::*;

mod app;
mod service;
mod state;

use crate::app::App;

fn main() {
    launch(App);
}
