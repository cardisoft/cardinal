use crate::window_controls::activate_window;
use base64::{Engine as _, engine::general_purpose};
use camino::Utf8Path;
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained, runtime::ProtocolObject,
};
use objc2_app_kit::{
    NSCursor, NSEvent, NSEventModifierFlags, NSEventType, NSImage, NSWindowDelegate,
};
use objc2_foundation::{
    NSData, NSInteger, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSURL,
};
use objc2_quick_look_ui::{
    QLPreviewItem, QLPreviewPanel, QLPreviewPanelDataSource, QLPreviewPanelDelegate,
};
use once_cell::unsync::OnceCell;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};
use tauri::{AppHandle, Emitter, Manager};

thread_local! {
    static PREVIEW_CONTROLLER: OnceCell<Retained<PreviewController>> = const { OnceCell::new() };
}

fn preview_controller(
    mtm: MainThreadMarker,
    panel: &Retained<QLPreviewPanel>,
) -> Retained<PreviewController> {
    PREVIEW_CONTROLLER.with(|cell| {
        cell.get_or_init(|| {
            let controller = PreviewController::new(mtm);
            let data_source = ProtocolObject::from_ref(&*controller);
            let delegate: &ProtocolObject<dyn QLPreviewPanelDelegate> =
                ProtocolObject::from_ref(&*controller);
            unsafe {
                panel.setDataSource(Some(data_source));
                panel.setDelegate(Some(delegate.as_ref()));
                panel.updateController();
            }
            controller
        })
        .clone()
    })
}

fn clear_preview_controller() {
    PREVIEW_CONTROLLER.with(|cell| {
        if let Some(x) = cell.get() {
            x.ivars().clear()
        }
    });
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct ScreenRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ScreenRect {
    fn to_nsrect(self) -> NSRect {
        NSRect::new(
            NSPoint::new(self.x, self.y),
            NSSize::new(self.width, self.height),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLookItemInput {
    pub path: String,
    pub rect: Option<ScreenRect>,
    pub transition_image: Option<String>,
}

const KEY_CODE_DOWN_ARROW: u16 = 125;
const KEY_CODE_UP_ARROW: u16 = 126;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuickLookKeyEvent {
    key_code: u16,
    characters: Option<String>,
    modifiers: QuickLookKeyModifiers,
}

#[derive(Debug, Clone, Serialize, Default)]
struct QuickLookKeyModifiers {
    shift: bool,
    control: bool,
    option: bool,
    command: bool,
}

struct PreviewItemState {
    url: Retained<NSURL>,
    title: Option<Retained<NSString>>,
}

impl QuickLookKeyEvent {
    fn from_event(event: &NSEvent) -> Option<Self> {
        let key_code = event.keyCode();
        let characters = event
            .charactersIgnoringModifiers()
            .map(|value| value.to_string());
        let flags = event.modifierFlags();
        Some(Self {
            key_code,
            characters,
            modifiers: QuickLookKeyModifiers::from_flags(flags),
        })
    }
}

impl QuickLookKeyModifiers {
    fn from_flags(flags: NSEventModifierFlags) -> Self {
        Self {
            shift: flags.contains(NSEventModifierFlags::Shift),
            control: flags.contains(NSEventModifierFlags::Control),
            option: flags.contains(NSEventModifierFlags::Option),
            command: flags.contains(NSEventModifierFlags::Command),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CardinalPreviewItem"]
    #[ivars = PreviewItemState]
    struct PreviewItemImpl;

    unsafe impl NSObjectProtocol for PreviewItemImpl {}
    unsafe impl QLPreviewItem for PreviewItemImpl {
        #[allow(non_snake_case)]
        #[unsafe(method_id(previewItemURL))]
        unsafe fn previewItemURL(&self) -> Option<Retained<NSURL>> {
            Some(self.ivars().url.clone())
        }

        #[allow(non_snake_case)]
        #[unsafe(method_id(previewItemTitle))]
        unsafe fn previewItemTitle(&self) -> Option<Retained<NSString>> {
            self.ivars().title.clone()
        }
    }
);

impl PreviewItemImpl {
    fn new(
        mtm: MainThreadMarker,
        url: Retained<NSURL>,
        title: Option<Retained<NSString>>,
    ) -> Retained<Self> {
        let obj = PreviewItemImpl::alloc(mtm).set_ivars(PreviewItemState { url, title });
        unsafe { msg_send![super(obj), init] }
    }
}

#[derive(Default)]
struct PreviewControllerState {
    items: RefCell<Vec<Retained<ProtocolObject<dyn QLPreviewItem>>>>,
    frames: RefCell<HashMap<String, ScreenRect>>,
    transitions: RefCell<HashMap<String, Retained<NSImage>>>,
    app_handle: RefCell<Option<AppHandle>>,
}

impl PreviewControllerState {
    fn clear(&self) {
        self.items.borrow_mut().clear();
        self.frames.borrow_mut().clear();
        self.transitions.borrow_mut().clear();
        self.app_handle.borrow_mut().take();
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CardinalPreviewController"]
    #[ivars = PreviewControllerState]
    struct PreviewController;

    unsafe impl NSObjectProtocol for PreviewController {}
    unsafe impl NSWindowDelegate for PreviewController {}
    unsafe impl QLPreviewPanelDataSource for PreviewController {
        #[allow(non_snake_case)]
        #[unsafe(method(numberOfPreviewItemsInPreviewPanel:))]
        fn numberOfPreviewItemsInPreviewPanel(&self, _panel: Option<&QLPreviewPanel>) -> NSInteger {
            self.ivars().items.borrow().len() as NSInteger
        }

        #[allow(non_snake_case)]
        #[unsafe(method_id(previewPanel:previewItemAtIndex:))]
        fn previewPanel_previewItemAtIndex(
            &self,
            _panel: Option<&QLPreviewPanel>,
            index: NSInteger,
        ) -> Option<Retained<ProtocolObject<dyn QLPreviewItem>>> {
            if index < 0 {
                None
            } else {
                let index = index as usize;
                self.ivars().items.borrow().get(index).cloned()
            }
        }
    }

    unsafe impl QLPreviewPanelDelegate for PreviewController {
        #[allow(non_snake_case)]
        #[unsafe(method(previewPanel:handleEvent:))]
        fn previewPanel_handleEvent(&self, _panel: &QLPreviewPanel, event: &NSEvent) -> bool {
            if event.r#type() != NSEventType::KeyDown {
                return false.into();
            }

            // Only handle Up/Down navigation to keep the payload surface minimal.
            if !matches!(event.keyCode(), KEY_CODE_DOWN_ARROW | KEY_CODE_UP_ARROW) {
                return false.into();
            }

            if let Some(app_handle) = self.ivars().app_handle.borrow().as_ref().cloned() {
                if let Some(payload) = QuickLookKeyEvent::from_event(event) {
                    let _ = app_handle.emit("quicklook-keydown", payload);
                }
            }

            true
        }

        #[allow(non_snake_case)]
        #[unsafe(method(previewPanel:sourceFrameOnScreenForPreviewItem:))]
        unsafe fn previewPanel_sourceFrameOnScreenForPreviewItem(
            &self,
            _panel: Option<&QLPreviewPanel>,
            item: Option<&ProtocolObject<dyn QLPreviewItem>>,
        ) -> NSRect {
            let Some(item) = item else {
                return NSRect::ZERO;
            };

            let url_opt = unsafe { item.previewItemURL() };
            let Some(url) = url_opt else {
                return NSRect::ZERO;
            };

            let path_opt = url.path();
            let Some(path_str) = path_opt else {
                return NSRect::ZERO;
            };
            let path = path_str.to_string();

            if let Some(rect) = self.ivars().frames.borrow().get(&path).copied() {
                return rect.to_nsrect();
            }

            NSRect::ZERO
        }

        #[allow(non_snake_case)]
        #[unsafe(method(previewPanel:transitionImageForPreviewItem:contentRect:))]
        fn previewPanel_transitionImageForPreviewItem_contentRect(
            &self,
            _panel: &QLPreviewPanel,
            item: &ProtocolObject<dyn QLPreviewItem>,
            _content_rect: *mut NSRect,
        ) -> *mut NSImage {
            let image: Option<*mut NSImage> = (|| {
                let url = unsafe { item.previewItemURL()? };
                let path = url.path()?.to_string();
                let transitions = self.ivars().transitions.borrow();
                transitions
                    .get(&path)
                    .map(|x| Retained::as_ptr(x).cast_mut())
            })();

            image.unwrap_or_default()
        }
    }
);

impl PreviewController {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let obj = PreviewController::alloc(mtm).set_ivars(PreviewControllerState::default());
        unsafe { msg_send![super(obj), init] }
    }
}

fn build_preview_item(
    mtm: MainThreadMarker,
    path: &str,
) -> Retained<ProtocolObject<dyn QLPreviewItem>> {
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    let title = Utf8Path::new(path)
        .file_name()
        .filter(|name| !name.is_empty())
        .map(NSString::from_str);

    let item = PreviewItemImpl::new(mtm, url, title);
    ProtocolObject::from_retained(item)
}

fn update_preview_items(
    mtm: MainThreadMarker,
    panel: &Retained<QLPreviewPanel>,
    app_handle: &AppHandle,
    items: Vec<QuickLookItemInput>,
) {
    let controller = preview_controller(mtm, panel);
    *controller.ivars().app_handle.borrow_mut() = Some(app_handle.clone());
    let preview_items: Vec<_> = items
        .iter()
        .map(|item| build_preview_item(mtm, &item.path))
        .collect();
    *controller.ivars().items.borrow_mut() = preview_items;

    let mut frames = controller.ivars().frames.borrow_mut();
    frames.clear();
    let mut transitions = controller.ivars().transitions.borrow_mut();
    transitions.clear();

    for item in items {
        if let Some(rect) = item.rect {
            frames.insert(item.path.clone(), rect);
        }
        if let Some(base64_data_uri) = &item.transition_image {
            if let Some(base64_data) = base64_data_uri.split(',').nth(1) {
                if let Ok(image_bytes) = general_purpose::STANDARD.decode(base64_data) {
                    let data = NSData::from_vec(image_bytes);
                    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                        transitions.insert(item.path.clone(), image);
                    }
                }
            }
        }
    }
    refresh_panel(panel);
}

fn refresh_panel(panel: &Retained<QLPreviewPanel>) {
    unsafe {
        panel.reloadData();
    }
}

fn shared_panel() -> Option<(MainThreadMarker, Retained<QLPreviewPanel>)> {
    let mtm = MainThreadMarker::new()?;
    let panel = unsafe { QLPreviewPanel::sharedPreviewPanel(mtm)? };
    Some((mtm, panel))
}

pub fn toggle_preview_panel(app_handle: AppHandle, items: Vec<QuickLookItemInput>) {
    let Some((mtm, panel)) = shared_panel() else {
        return;
    };

    if panel.isVisible() {
        close_preview_panel(app_handle);
        return;
    }

    update_preview_items(mtm, &panel, &app_handle, items);
    NSCursor::setHiddenUntilMouseMoves(false);
    panel.orderFront(None);
}

pub fn update_preview_panel(app_handle: AppHandle, items: Vec<QuickLookItemInput>) {
    let Some((mtm, panel)) = shared_panel() else {
        return;
    };

    if !panel.isVisible() {
        return;
    }

    update_preview_items(mtm, &panel, &app_handle, items);
}

pub fn close_preview_panel(app_handle: AppHandle) {
    if let Some((_, panel)) = shared_panel() {
        if panel.isVisible() {
            panel.close();
            if let Some(window) = app_handle.get_webview_window("main") {
                activate_window(&window);
            }
        }
    };
    clear_preview_controller();
}
