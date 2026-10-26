use crate::commands::SearchState;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};
use tracing::{error, info, warn};

/// Whether the main window is currently visible and focused.
///
/// Written on the main thread (window focus events in `lib.rs`, plus the show/hide helpers
/// below) and read by the background loop. The background loop used to ask Tauri's
/// `is_visible()/is_focused()/is_minimized()` getters instead; each of those sends a message to
/// the main event loop and blocks the calling thread until it is serviced, so a busy main
/// thread stalled the indexing loop (and a hide-flush decision could be delayed indefinitely).
static MAIN_WINDOW_FOREGROUND: AtomicBool = AtomicBool::new(false);

pub fn set_main_window_foreground(foreground: bool) {
    MAIN_WINDOW_FOREGROUND.store(foreground, Ordering::Release);
}

pub fn main_window_is_foreground() -> bool {
    MAIN_WINDOW_FOREGROUND.load(Ordering::Acquire)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowToggle {
    Hidden,
    Shown,
    Failed,
}

pub fn activate_window<R: Runtime>(window: &WebviewWindow<R>) {
    if let Ok(true) = window.is_minimized()
        && let Err(err) = window.unminimize()
    {
        error!(?err, "Failed to unminimize window");
    }

    if let Ok(false) = window.is_visible()
        && let Err(err) = window.show()
    {
        error!(?err, "Failed to show window");
    }

    if let Err(err) = window.set_focus() {
        error!(?err, "Failed to focus window");
    } else {
        // The Focused(true) window event will confirm this shortly; set it eagerly so a
        // hide-flush countdown started before activation is cancelled right away.
        set_main_window_foreground(true);
    }
}

pub fn hide_window<R: Runtime>(window: &WebviewWindow<R>) -> bool {
    if let Err(err) = window.hide() {
        error!(?err, "Failed to hide window");
        return false;
    }
    set_main_window_foreground(false);
    true
}

pub fn trigger_quick_launch<R: Runtime>(window: &WebviewWindow<R>) {
    activate_window(window);

    if let Err(err) = window.emit("quick_launch", ()) {
        error!(?err, "Failed to emit quick launch event");
    }
}

pub fn toggle_window<R: Runtime>(window: &WebviewWindow<R>) -> WindowToggle {
    let is_visible = window.is_visible().unwrap_or(true);
    let is_minimized = window.is_minimized().unwrap_or(false);
    let is_focused = window.is_focused().unwrap_or(false);

    if is_visible && !is_minimized && is_focused {
        if hide_window(window) {
            WindowToggle::Hidden
        } else {
            WindowToggle::Failed
        }
    } else {
        trigger_quick_launch(window);
        WindowToggle::Shown
    }
}

pub fn activate_main_window_impl(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        activate_window(&window);
        info!("Main window activated via command");
        if let Some(state) = app.try_state::<SearchState>() {
            let _ = state.update_window_state_tx.try_send(());
        }
    } else {
        warn!("Activate requested but main window is unavailable");
    }
}

pub fn hide_main_window_impl(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        hide_window(&window);
        info!("Main window hidden via command");
        if let Some(state) = app.try_state::<SearchState>() {
            let _ = state.update_window_state_tx.try_send(());
        }
    }
}

pub fn toggle_main_window_impl(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if matches!(toggle_window(&window), WindowToggle::Hidden) {
            info!("Main window hidden via command");
        } else {
            info!("Main window shown via command");
        }
        if let Some(state) = app.try_state::<SearchState>() {
            let _ = state.update_window_state_tx.try_send(());
        }
    } else {
        warn!("Toggle requested but main window is unavailable");
    }
}
