mod actions;
pub mod app;
mod input;
pub mod keymap;
mod render;
mod sort;
mod state;

use anyhow::Result;
use app::AppRuntime;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use input::run_app;
use keymap::Keymap;
use std::io::stdout;

pub fn run(runtime: &AppRuntime) -> Result<()> {
    run_with_options(runtime, true, Keymap::default())
}

pub fn run_with_options(runtime: &AppRuntime, confirm_quit: bool, keymap: Keymap) -> Result<()> {
    execute!(stdout(), EnableMouseCapture)?;
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, runtime, confirm_quit, keymap);
    ratatui::restore();
    execute!(stdout(), DisableMouseCapture)?;
    result
}
