use crate::commands::SearchState;
use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowToggle {
    Hidden,
    Shown,
    Failed,
}

pub fn activate_window<R: Runtime>(window: &WebviewWindow<R>) {
    let window_for_task = window.clone();
    if let Err(err) = window.run_on_main_thread(move || {
        apply_active_space_behavior(&window_for_task);
        show_and_focus_window(&window_for_task);
    }) {
        error!(?err, "Failed to schedule macOS window activation");
        show_and_focus_window(window);
    }
}

fn show_and_focus_window<R: Runtime>(window: &WebviewWindow<R>) {
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
    }
}

fn apply_active_space_behavior<R: Runtime>(window: &WebviewWindow<R>) {
    let ns_window = match window.ns_window() {
        Ok(ns_window) => ns_window,
        Err(err) => {
            error!(?err, "Failed to get macOS NSWindow");
            return;
        }
    };

    if ns_window.is_null() {
        warn!("macOS NSWindow pointer is null");
        return;
    }

    // Move the existing window to whichever Space is active before AppKit focuses it.
    let ns_window = unsafe { &*ns_window.cast::<NSWindow>() };
    let behavior = collection_behavior_for_active_space(ns_window.collectionBehavior());
    ns_window.setCollectionBehavior(behavior);
}

fn collection_behavior_for_active_space(
    mut behavior: NSWindowCollectionBehavior,
) -> NSWindowCollectionBehavior {
    behavior.remove(NSWindowCollectionBehavior::CanJoinAllSpaces);
    behavior.insert(NSWindowCollectionBehavior::MoveToActiveSpace);
    behavior
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

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_app_kit::NSWindowCollectionBehavior;

    #[test]
    fn active_space_behavior_moves_window_without_joining_all_spaces() {
        let behavior = collection_behavior_for_active_space(NSWindowCollectionBehavior::Default);

        assert!(behavior.contains(NSWindowCollectionBehavior::MoveToActiveSpace));
        assert!(!behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
    }

    #[test]
    fn active_space_behavior_preserves_unrelated_flags() {
        let existing =
            NSWindowCollectionBehavior::Managed | NSWindowCollectionBehavior::ParticipatesInCycle;

        let behavior = collection_behavior_for_active_space(existing);

        assert!(behavior.contains(NSWindowCollectionBehavior::Managed));
        assert!(behavior.contains(NSWindowCollectionBehavior::ParticipatesInCycle));
        assert!(behavior.contains(NSWindowCollectionBehavior::MoveToActiveSpace));
    }

    #[test]
    fn active_space_behavior_clears_all_spaces_visibility() {
        let existing =
            NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Managed;

        let behavior = collection_behavior_for_active_space(existing);

        assert!(!behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(behavior.contains(NSWindowCollectionBehavior::MoveToActiveSpace));
        assert!(behavior.contains(NSWindowCollectionBehavior::Managed));
    }
}
