use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tauri::{AppHandle, Emitter};
use tracing::error;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLifecycleState {
    Initializing = 0,
    Updating = 1,
    Ready = 2,
}

impl AppLifecycleState {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Initializing,
            1 => Self::Updating,
            2 => Self::Ready,
            _ => Self::Initializing,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Initializing => "Initializing",
            Self::Updating => "Updating",
            Self::Ready => "Ready",
        }
    }
}

static APP_LIFECYCLE_STATE: AtomicU8 = AtomicU8::new(AppLifecycleState::Initializing as u8);

pub static APP_QUIT: AtomicBool = AtomicBool::new(false);
pub static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn load_app_state() -> AppLifecycleState {
    AppLifecycleState::from_u8(APP_LIFECYCLE_STATE.load(Ordering::Acquire))
}

pub fn store_app_state(state: AppLifecycleState) {
    APP_LIFECYCLE_STATE.store(state as u8, Ordering::Release);
}

pub fn emit_app_state(app_handle: &AppHandle) {
    if let Err(err) = app_handle.emit("app_lifecycle_state", load_app_state().as_str()) {
        error!("Failed to emit app_lifecycle_state event: {:?}", err);
    }
}

pub fn update_app_state(app_handle: &AppHandle, state: AppLifecycleState) {
    if load_app_state() == state {
        return;
    }
    store_app_state(state);
    emit_app_state(app_handle);
}
