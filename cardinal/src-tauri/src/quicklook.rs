use block2::RcBlock;
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{NSEvent, NSEventMask, NSEventType, NSWindow};
use objc2_foundation::{NSInteger, NSObject, NSObjectProtocol, NSString, NSURL};
use objc2_quick_look_ui::{QLPreviewPanel, QLPreviewPanelDataSource};
use std::{
    ffi::c_void,
    ptr::{self, NonNull},
    sync::{Mutex, OnceLock},
};

// Thread-safe wrapper
// SAFETY: This wrapper is only used for Retained<AnyObject> from the event monitor.
// According to Apple documentation,
// NSEvent monitor objects are thread-safe and can be safely shared across threads.
// Therefore, it is safe to implement Send and Sync for this specific usage.
struct SendSyncWrapper<T>(T);
unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}

enum PanelAction {
    Update,
    Toggle,
    Open,
}

#[derive(Default)]
struct State {
    url: Mutex<Option<Retained<NSURL>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "SimpleQLDataSource"]
    #[ivars = State]
    struct DataSource;

    unsafe impl NSObjectProtocol for DataSource {}
    unsafe impl QLPreviewPanelDataSource for DataSource {}

    impl DataSource {
        #[unsafe(method(numberOfPreviewItemsInPreviewPanel:))]
        fn count(&self, _panel: &QLPreviewPanel) -> NSInteger {
            self.ivars().url.lock().unwrap().is_some() as NSInteger
        }

        #[unsafe(method_id(previewPanel:previewItemAtIndex:))]
        fn item(&self, _panel: &QLPreviewPanel, _idx: NSInteger) -> Option<Retained<NSURL>> {
            self.ivars().url.lock().unwrap().clone()
        }
    }
);

static DELEGATE: OnceLock<Retained<DataSource>> = OnceLock::new();
static EVENT_MONITOR: Mutex<Option<SendSyncWrapper<Retained<AnyObject>>>> = Mutex::new(None);
static INTERCEPT_MODE: Mutex<bool> = Mutex::new(true);

fn get_or_create_delegate() -> Retained<DataSource> {
    DELEGATE
        .get_or_init(|| {
            let obj = DataSource::alloc().set_ivars(State::default());
            unsafe { msg_send![super(obj), init] }
        })
        .clone()
}

fn set_delegate_url(path: &str) -> Retained<DataSource> {
    let delegate: Retained<DataSource> = get_or_create_delegate();
    let url: Retained<NSURL> = NSURL::fileURLWithPath(&NSString::from_str(path));
    *delegate.ivars().url.lock().unwrap() = Some(url);
    delegate
}

fn get_panel(mtm: MainThreadMarker) -> Option<Retained<QLPreviewPanel>> {
    unsafe { QLPreviewPanel::sharedPreviewPanel(mtm) }
}

fn setup_panel(delegate: &Retained<DataSource>, panel: &Retained<QLPreviewPanel>) {
    let ds_protocol: &ProtocolObject<dyn QLPreviewPanelDataSource> =
        ProtocolObject::<dyn QLPreviewPanelDataSource>::from_ref(&**delegate);
    unsafe {
        panel.setDataSource(Some(ds_protocol));
        panel.reloadData();
    }
}

fn handle_preview_panel(path: &str, main_window: *mut c_void, action: PanelAction) -> bool {
    let Some(mtm): Option<MainThreadMarker> = MainThreadMarker::new() else {
        return false;
    };

    unsafe {
        _ensure_event_monitor(main_window);
    }
    _enable_interception();

    let Some(panel): Option<Retained<QLPreviewPanel>> = get_panel(mtm) else {
        return false;
    };

    match action {
        PanelAction::Update => {
            if panel.isVisible() {
                let delegate: Retained<DataSource> = set_delegate_url(path);
                setup_panel(&delegate, &panel);
            }
            false
        }
        PanelAction::Toggle => unsafe {
            if panel.isVisible() {
                panel.close();
                if !main_window.is_null() {
                    let mw: &NSWindow = &*(main_window as *const NSWindow);
                    mw.makeKeyWindow();
                }
                false
            } else {
                let delegate: Retained<DataSource> = set_delegate_url(path);
                setup_panel(&delegate, &panel);
                panel.makeKeyAndOrderFront(None);
                true
            }
        },
        PanelAction::Open => {
            let delegate: Retained<DataSource> = set_delegate_url(path);
            setup_panel(&delegate, &panel);
            panel.makeKeyAndOrderFront(None);
            true
        }
    }
}

fn _enable_interception() {
    if let Ok(mut guard) = INTERCEPT_MODE.lock() {
        *guard = true;
    }
}

unsafe fn _ensure_event_monitor(main_window: *mut c_void) {
    let mut monitor_guard: std::sync::MutexGuard<'_, Option<SendSyncWrapper<Retained<AnyObject>>>> =
        match EVENT_MONITOR.lock().ok() {
            Some(guard) => guard,
            None => return,
        };

    if monitor_guard.is_none() || main_window.is_null() {
        return;
    }

    let main_window_addr: usize = main_window as usize;

    let handler = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        let mtm: MainThreadMarker = unsafe { MainThreadMarker::new_unchecked() };
        unsafe {
            let event_ref: &NSEvent = event.as_ref();
            let event_type: NSEventType = event_ref.r#type();

            // Handle left mouse down - disable interception if clicking on panel
            if event_type == NSEventType::LeftMouseDown {
                if let (Some(win), Some(panel)) = (
                    event_ref.window(mtm),
                    QLPreviewPanel::sharedPreviewPanel(mtm),
                ) {
                    if Retained::as_ptr(&win) as *const AnyObject
                        == Retained::as_ptr(&panel) as *const AnyObject
                    {
                        if let Ok(mut guard) = INTERCEPT_MODE.lock() {
                            *guard = false;
                        }
                    }
                }
                return event.as_ptr();
            }

            // Handle key down - forward specific keys to main window
            if event_type == NSEventType::KeyDown {
                if let Some(panel) = QLPreviewPanel::sharedPreviewPanel(mtm) {
                    let is_key: bool = msg_send![&panel, isKeyWindow];
                    if panel.isVisible() && is_key {
                        let should_intercept: bool =
                            INTERCEPT_MODE.lock().ok().map(|b| *b).unwrap_or(true);
                        if should_intercept {
                            let key_code: u16 = event_ref.keyCode();
                            // 126=Up, 125=Down, 49=Space
                            if [126, 125, 49].contains(&key_code) {
                                (*(main_window_addr as *const NSWindow)).keyDown(event_ref);
                                return ptr::null_mut();
                            }
                        }
                    }
                }
            }
            event.as_ptr()
        }
    });

    let mask: NSEventMask = NSEventMask::KeyDown.union(NSEventMask::LeftMouseDown);
    if let Some(monitor) =
        unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &handler) }
    {
        *monitor_guard = Some(SendSyncWrapper(monitor));
    }
}

pub fn update(path: &str, main_window: *mut c_void) -> bool {
    handle_preview_panel(path, main_window, PanelAction::Update)
}

pub fn toggle(path: &str, main_window: *mut c_void) -> bool {
    handle_preview_panel(path, main_window, PanelAction::Toggle)
}

pub fn open(path: &str, main_window: *mut c_void) -> bool {
    handle_preview_panel(path, main_window, PanelAction::Open)
}
