use crate::commands::SearchState;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowToggle {
    Hidden,
    Shown,
    Failed,
}

/// Moves the window back onto a monitor when its restored position lies completely outside
/// every available screen. The window-state plugin can restore coordinates saved on a display
/// that is no longer attached, which makes the app look like it never launched.
pub(crate) fn ensure_on_screen<R: Runtime>(window: &WebviewWindow<R>) {
    let Ok(position) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let Ok(monitors) = window.available_monitors() else {
        return;
    };
    if monitors.is_empty() {
        return;
    }

    let visible = monitors.iter().any(|monitor| {
        let origin = monitor.position();
        let extent = monitor.size();
        overlaps(
            position.x,
            position.y,
            size.width as i32,
            size.height as i32,
            origin.x,
            origin.y,
            extent.width as i32,
            extent.height as i32,
        )
    });
    if visible {
        return;
    }

    // Nothing overlaps: park the window near the top-left of the primary monitor.
    let monitor = window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| monitors.into_iter().next());
    let Some(monitor) = monitor else {
        return;
    };
    let origin = monitor.position();
    if let Err(err) =
        window.set_position(tauri::PhysicalPosition::new(origin.x + 40, origin.y + 40))
    {
        error!(?err, "Failed to move off-screen window back into view");
    }
}

/// True when two rectangles share at least one pixel.
fn overlaps(ax: i32, ay: i32, aw: i32, ah: i32, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

/// macOS: makes a window appear on the Space that is active when it is shown.
///
/// By default AppKit re-orders a hidden window on the Space where it was last visible, so an app
/// hidden on Desktop 2 and reopened from the Dock while Desktop 1 is active drags the user back
/// to Desktop 2. `NSWindowCollectionBehaviorMoveToActiveSpace` tells AppKit to move the window to
/// the active Space when it is ordered front while hidden, which is exactly the reopen path here.
///
/// Only the show-after-hide path is affected: a window that is already visible keeps macOS's
/// normal "switch to that Space" behaviour, which users expect when they click an app's window.
#[cfg(target_os = "macos")]
pub(crate) fn prefer_active_space<R: Runtime>(window: &WebviewWindow<R>) {
    let Ok(raw) = window.ns_window() else {
        return;
    };
    if raw.is_null() {
        return;
    }

    // SAFETY: `ns_window()` returns the live `NSWindow` that Tauri owns for this webview window;
    // it outlives this call, and we only read/write a collection-behaviour bit mask.
    unsafe {
        let ns_window = &*raw.cast::<objc2_app_kit::NSWindow>();
        let behavior = ns_window.collectionBehavior()
            | objc2_app_kit::NSWindowCollectionBehavior::MoveToActiveSpace;
        ns_window.setCollectionBehavior(behavior);
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn prefer_active_space<R: Runtime>(_window: &WebviewWindow<R>) {}

pub fn activate_window<R: Runtime>(window: &WebviewWindow<R>) {
    if let Ok(true) = window.is_minimized()
        && let Err(err) = window.unminimize()
    {
        error!(?err, "Failed to unminimize window");
    }

    if let Ok(false) = window.is_visible() {
        // Must be set while the window is hidden: AppKit only honours it when the window is
        // ordered front from a non-visible state.
        prefer_active_space(window);
        if let Err(err) = window.show() {
            error!(?err, "Failed to show window");
        }
    }

    ensure_on_screen(window);

    if let Err(err) = window.set_focus() {
        error!(?err, "Failed to focus window");
    }
}

pub fn hide_window<R: Runtime>(window: &WebviewWindow<R>) -> bool {
    if let Err(err) = window.hide() {
        error!(?err, "Failed to hide window");
        return false;
    }
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

pub fn is_main_window_foreground(app_handle: &AppHandle) -> bool {
    let Some(window) = app_handle.get_webview_window("main") else {
        return false;
    };

    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    let minimized = window.is_minimized().unwrap_or(false);

    visible && focused && !minimized
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

/// Hides the bookmarks window together with the main window.
///
/// The bookmarks panel is a separate OS window now, so hiding the app (Esc, tray, quick
/// launch) must not leave it floating on screen.
fn hide_bookmarks_window_too(app: &AppHandle) {
    if let Some(bookmarks) = app.get_webview_window(crate::bookmarks_window::BOOKMARKS_WINDOW_LABEL)
    {
        let _ = bookmarks.hide();
    }
}

pub fn hide_main_window_impl(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if hide_window(&window) {
            hide_bookmarks_window_too(app);
        }
        info!("Main window hidden via command");
        if let Some(state) = app.try_state::<SearchState>() {
            let _ = state.update_window_state_tx.try_send(());
        }
    }
}

pub fn toggle_main_window_impl(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if matches!(toggle_window(&window), WindowToggle::Hidden) {
            hide_bookmarks_window_too(app);
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
