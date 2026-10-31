//! Standalone window for the bookmarks manager.
//!
//! The panel used to be a DOM overlay inside the main webview, which meant it could never be
//! moved outside the application window and therefore always covered part of the result list.
//! A real window is draggable anywhere on screen (including onto another display) and survives
//! the main window being resized or hidden.

use crate::window_controls::{ensure_on_screen, prefer_active_space};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tracing::info;

/// Window label. Also the id used by the capability file and the window-state plugin.
pub const BOOKMARKS_WINDOW_LABEL: &str = "bookmarks";

const INITIAL_WIDTH: f64 = 640.0;
const INITIAL_HEIGHT: f64 = 760.0;
const MIN_WIDTH: f64 = 420.0;
const MIN_HEIGHT: f64 = 260.0;

/// Default landing position: right-hand side of the primary monitor, leaving the results list
/// visible to its left.
fn initial_position(app: &AppHandle, width: f64) -> (f64, f64) {
    let Some(monitor) = app.primary_monitor().ok().flatten() else {
        return (120.0, 120.0);
    };
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let origin = monitor.position().to_logical::<f64>(scale);
    let margin = 48.0;
    (
        origin.x + (size.width - width - margin).max(margin),
        origin.y + margin,
    )
}

/// Creates the window if it does not exist yet, then shows and focuses it.
fn create_or_show(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(BOOKMARKS_WINDOW_LABEL) {
        // Show it on whichever Desktop the user is on now, not the one it was last seen on.
        prefer_active_space(&window);
        window.show()?;
        window.unminimize().ok();
        window.set_focus()?;
        ensure_on_screen(&window);
        return Ok(window);
    }

    let (x, y) = initial_position(app, INITIAL_WIDTH);
    let window = WebviewWindowBuilder::new(
        app,
        BOOKMARKS_WINDOW_LABEL,
        // Same bundle, different entry: `main.tsx` looks at this parameter.
        WebviewUrl::App("index.html?window=bookmarks".into()),
    )
    // Frameless: the panel paints its own card, and the header drags the OS window.
    .decorations(false)
    .shadow(true)
    .resizable(true)
    .inner_size(INITIAL_WIDTH, INITIAL_HEIGHT)
    .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
    .position(x, y)
    .title("Cardinal")
    .visible(false)
    .build()?;

    ensure_on_screen(&window);
    prefer_active_space(&window);
    window.show()?;
    window.set_focus()?;
    info!("Bookmarks window created");
    Ok(window)
}

#[tauri::command]
pub async fn show_bookmarks_window(app: AppHandle) -> Result<(), String> {
    create_or_show(&app).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hide_bookmarks_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window(BOOKMARKS_WINDOW_LABEL) {
        let _ = window.hide();
    }
}

/// Keyboard shortcut / menu entry: first press opens, later presses toggle visibility.
#[tauri::command]
pub async fn toggle_bookmarks_window(app: AppHandle) -> Result<(), String> {
    match app.get_webview_window(BOOKMARKS_WINDOW_LABEL) {
        Some(window) => {
            if window.is_visible().unwrap_or(false) {
                window.hide().map_err(|e| e.to_string())?;
            } else {
                prefer_active_space(&window);
                window.show().map_err(|e| e.to_string())?;
                window.set_focus().map_err(|e| e.to_string())?;
                ensure_on_screen(&window);
            }
            Ok(())
        }
        None => create_or_show(&app).map(|_| ()).map_err(|e| e.to_string()),
    }
}
