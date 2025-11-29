use crate::window_controls::trigger_quick_launch;
use camino::Utf8Path;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::NSWindowDelegate;
use objc2_foundation::{NSInteger, NSNotification, NSObject, NSObjectProtocol, NSString, NSURL};
use objc2_quick_look_ui::{
    QLPreviewItem, QLPreviewPanel, QLPreviewPanelDataSource, QLPreviewPanelDelegate,
};
use std::cell::RefCell;
use tauri::{AppHandle, Manager};

thread_local! {
    static PREVIEW_CONTROLLER: RefCell<Option<Retained<PreviewController>>> = const { RefCell::new(None) };
}

fn clear_preview_controller() {
    PREVIEW_CONTROLLER.with(|cell| {
        cell.borrow_mut().take();
    });
}

struct PreviewItemState {
    url: Retained<NSURL>,
    title: Option<Retained<NSString>>,
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
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CardinalPreviewController"]
    #[ivars = PreviewControllerState]
    struct PreviewController;

    unsafe impl NSObjectProtocol for PreviewController {}
    unsafe impl NSWindowDelegate for PreviewController {
        #[allow(non_snake_case)]
        #[unsafe(method(windowWillClose:))]
        unsafe fn windowWillClose(&self, _notification: &NSNotification) {
            clear_preview_controller();
        }
    }
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

    unsafe impl QLPreviewPanelDelegate for PreviewController {}
);

impl PreviewController {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let obj = PreviewController::alloc(mtm).set_ivars(PreviewControllerState::default());
        unsafe { msg_send![super(obj), init] }
    }
}

fn preview_controller(mtm: MainThreadMarker) -> Retained<PreviewController> {
    PREVIEW_CONTROLLER.with(|cell| {
        if let Some(controller) = cell.borrow().as_ref() {
            return controller.clone();
        }

        let controller = PreviewController::new(mtm);
        cell.borrow_mut().replace(controller.clone());
        controller
    })
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

fn build_preview_items(
    mtm: MainThreadMarker,
    paths: Vec<String>,
) -> Vec<Retained<ProtocolObject<dyn QLPreviewItem>>> {
    paths
        .into_iter()
        .map(|path| build_preview_item(mtm, &path))
        .collect()
}

fn set_preview_items(mtm: MainThreadMarker, paths: Vec<String>) {
    let controller = preview_controller(mtm);
    let items = build_preview_items(mtm, paths);
    *controller.ivars().items.borrow_mut() = items;
}

fn setup_panel(mtm: MainThreadMarker, panel: &Retained<QLPreviewPanel>) {
    let controller = preview_controller(mtm);
    let data_source = ProtocolObject::from_ref(&*controller);
    let delegate: &ProtocolObject<dyn QLPreviewPanelDelegate> =
        ProtocolObject::from_ref(&*controller);
    let has_items = !controller.ivars().items.borrow().is_empty();

    unsafe {
        panel.setDataSource(Some(data_source));
        panel.setDelegate(Some(delegate.as_ref()));
        panel.updateController();
        panel.reloadData();
        if has_items {
            panel.setCurrentPreviewItemIndex(0);
            panel.refreshCurrentPreviewItem();
        }
    }
}

fn shared_panel() -> Option<(MainThreadMarker, Retained<QLPreviewPanel>)> {
    let mtm = MainThreadMarker::new()?;
    let panel = unsafe { QLPreviewPanel::sharedPreviewPanel(mtm)? };
    Some((mtm, panel))
}

pub fn open_preview_panel(paths: Vec<String>) {
    let Some((mtm, panel)) = shared_panel() else {
        return;
    };

    set_preview_items(mtm, paths);
    setup_panel(mtm, &panel);
    panel.makeKeyAndOrderFront(None);
}

pub fn update_preview_panel(paths: Vec<String>) {
    let Some((mtm, panel)) = shared_panel() else {
        return;
    };

    if !panel.isVisible() {
        clear_preview_controller();
        return;
    }

    set_preview_items(mtm, paths);
    setup_panel(mtm, &panel);
}

pub fn close_preview_panel(app_handle: AppHandle) {
    let Some((_, panel)) = shared_panel() else {
        clear_preview_controller();
        return;
    };

    if panel.isVisible() {
        panel.close();
        if let Some(window) = app_handle.get_webview_window("main") {
            trigger_quick_launch(&window);
        }
    }

    clear_preview_controller();
}
