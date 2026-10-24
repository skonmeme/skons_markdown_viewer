//! A document window: the web view showing the rendered document, the find
//! bar and the bridge to the page script (STANDARD §4, §5 and §6).

use std::cell::{Cell, OnceCell, RefCell};

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, Bool, NSObjectProtocol, ProtocolObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSBeep, NSColor, NSControl, NSControlStateValueOff,
    NSControlStateValueOn, NSControlTextEditingDelegate, NSDocument, NSDocumentController,
    NSEventModifierFlags, NSImage, NSMenuItem, NSMenuItemValidation, NSPrintInfo, NSPrintOperation,
    NSPrintingPaginationMode, NSResponder, NSSearchFieldDelegate, NSTextFieldDelegate, NSTextView,
    NSView, NSWindow, NSWindowController, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{
    NSCopying, NSDictionary, NSError, NSNumber, NSObject, NSPoint, NSRect, NSSize, NSString, NSURL,
    NSURLRequest,
};
use objc2_web_kit::{
    WKContentWorld, WKNavigation, WKNavigationAction, WKNavigationActionPolicy,
    WKNavigationDelegate, WKNavigationType, WKUserScript, WKUserScriptInjectionTime, WKWebView,
    WKWebViewConfiguration,
};
use smv_core::page::{CONTENT_WORLD, VIEWER_JS};
use smv_core::settings::{ColorMode, MarginMode};

use crate::document::Document;
use crate::find_bar::FindBar;
use crate::scheme::SchemeHandler;
use crate::settings_store;
use crate::util::{self, ns};

const FRAME_AUTOSAVE_NAME: &str = "SMVDocumentWindow";
const TABBING_IDENTIFIER: &str = "SMVDocumentWindow";

thread_local! {
    /// Top-left point for the next cascaded window.
    static CASCADE_POINT: Cell<NSPoint> = const { Cell::new(NSPoint::new(0.0, 0.0)) };
}

type JsCompletion = Box<dyn Fn(Option<&AnyObject>)>;

pub struct ViewerIvars {
    document: Weak<Document>,
    web_view: OnceCell<Retained<WKWebView>>,
    world: OnceCell<Retained<WKContentWorld>>,
    find_bar: OnceCell<FindBar>,
    /// Whether the page finished loading and the page script is available.
    loaded: Cell<bool>,
    content_version: Cell<u64>,
    page_content_version: Cell<u64>,
    style_version: Cell<u64>,
    page_style_version: Cell<u64>,
    pending_scroll: Cell<Option<f64>>,
    pending_fragment: RefCell<Option<String>>,
    /// Query of the last search, kept when the find bar is closed.
    find_query: RefCell<String>,
}

define_class!(
    // SAFETY: NSWindowController may be subclassed; the designated
    // initializer is called with the ivars set. No Drop impl.
    #[unsafe(super(NSWindowController, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SMVViewerController"]
    #[ivars = ViewerIvars]
    pub struct ViewerController;

    unsafe impl NSObjectProtocol for ViewerController {}

    unsafe impl WKNavigationDelegate for ViewerController {
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn decide_policy(
            &self,
            _web_view: &WKWebView,
            action: &WKNavigationAction,
            decision_handler: &block2::DynBlock<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            decision_handler.call((self.navigation_policy(action),));
        }

        #[unsafe(method(webView:didFinishNavigation:))]
        fn did_finish_navigation(&self, _web_view: &WKWebView, _navigation: Option<&WKNavigation>) {
            self.page_did_load();
        }

        #[unsafe(method(webViewWebContentProcessDidTerminate:))]
        fn web_content_process_did_terminate(&self, _web_view: &WKWebView) {
            self.load_page();
        }
    }

    unsafe impl NSControlTextEditingDelegate for ViewerController {
        #[unsafe(method(control:textView:doCommandBySelector:))]
        fn control_do_command(&self, _control: &NSControl, _view: &NSTextView, command: Sel) -> bool {
            if command == sel!(insertNewline:) {
                let shift = NSApplication::sharedApplication(self.mtm())
                    .currentEvent()
                    .is_some_and(|e| e.modifierFlags().contains(NSEventModifierFlags::Shift));
                self.step_find(shift);
                true
            } else if command == sel!(cancelOperation:) {
                self.close_find();
                true
            } else {
                false
            }
        }
    }

    unsafe impl NSTextFieldDelegate for ViewerController {}

    unsafe impl NSSearchFieldDelegate for ViewerController {}

    unsafe impl NSMenuItemValidation for ViewerController {
        #[unsafe(method(validateMenuItem:))]
        fn validate_menu_item(&self, item: &NSMenuItem) -> bool {
            self.validate(item)
        }
    }

    impl ViewerController {
        #[unsafe(method(smvReload:))]
        fn reload_action(&self, _sender: Option<&AnyObject>) {
            if let Some(document) = self.document() {
                document.reload_from_disk(true);
            }
            self.reload_page_keeping_scroll();
        }

        #[unsafe(method(smvZoomIn:))]
        fn zoom_in_action(&self, _sender: Option<&AnyObject>) {
            settings_store::update(self.mtm(), |s| s.zoom_in());
        }

        #[unsafe(method(smvZoomOut:))]
        fn zoom_out_action(&self, _sender: Option<&AnyObject>) {
            settings_store::update(self.mtm(), |s| s.zoom_out());
        }

        #[unsafe(method(smvActualSize:))]
        fn actual_size_action(&self, _sender: Option<&AnyObject>) {
            settings_store::update(self.mtm(), |s| s.zoom = 1.0);
        }

        #[unsafe(method(smvIncreaseMargins:))]
        fn increase_margins_action(&self, _sender: Option<&AnyObject>) {
            let width = self.viewport_width();
            settings_store::update(self.mtm(), |s| s.step_margin(width, 1));
        }

        #[unsafe(method(smvDecreaseMargins:))]
        fn decrease_margins_action(&self, _sender: Option<&AnyObject>) {
            let width = self.viewport_width();
            settings_store::update(self.mtm(), |s| s.step_margin(width, -1));
        }

        #[unsafe(method(smvAutomaticMargins:))]
        fn automatic_margins_action(&self, _sender: Option<&AnyObject>) {
            settings_store::update(self.mtm(), |s| s.margin_mode = MarginMode::Automatic);
        }

        #[unsafe(method(smvShowFind:))]
        fn show_find_action(&self, _sender: Option<&AnyObject>) {
            self.show_find(true);
        }

        #[unsafe(method(smvFindNext:))]
        fn find_next_action(&self, _sender: Option<&AnyObject>) {
            self.find_again(false);
        }

        #[unsafe(method(smvFindPrevious:))]
        fn find_previous_action(&self, _sender: Option<&AnyObject>) {
            self.find_again(true);
        }

        #[unsafe(method(smvUseSelectionForFind:))]
        fn use_selection_for_find_action(&self, _sender: Option<&AnyObject>) {
            let weak = Weak::from(self);
            self.call_js(
                "return smv.selectionText();",
                &[],
                Some(Box::new(move |result| {
                    let (Some(this), Some(text)) = (weak.load(), string_value(result)) else {
                        return;
                    };
                    if text.is_empty() {
                        return;
                    }
                    this.find_bar().set_query(&text);
                    this.show_find(false);
                    this.run_find();
                })),
            );
        }

        #[unsafe(method(smvFindFieldChanged:))]
        fn find_field_changed_action(&self, _sender: Option<&AnyObject>) {
            self.run_find();
        }

        #[unsafe(method(smvFindNavigate:))]
        fn find_navigate_action(&self, _sender: Option<&AnyObject>) {
            let backwards = self.find_bar().selected_navigation() == 0;
            self.step_find(backwards);
        }

        #[unsafe(method(smvFindOptionsChanged:))]
        fn find_options_changed_action(&self, _sender: Option<&AnyObject>) {
            self.run_find();
        }

        #[unsafe(method(smvCloseFind:))]
        fn close_find_action(&self, _sender: Option<&AnyObject>) {
            self.close_find();
        }
    }
);

impl ViewerController {
    pub fn new(mtm: MainThreadMarker, document: &Document) -> Retained<Self> {
        let window = make_window(mtm);
        let this = Self::alloc(mtm).set_ivars(ViewerIvars {
            document: Weak::from(document),
            web_view: OnceCell::new(),
            world: OnceCell::new(),
            find_bar: OnceCell::new(),
            loaded: Cell::new(false),
            content_version: Cell::new(0),
            page_content_version: Cell::new(0),
            style_version: Cell::new(0),
            page_style_version: Cell::new(0),
            pending_scroll: Cell::new(None),
            pending_fragment: RefCell::new(None),
            find_query: RefCell::new(String::new()),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithWindow: &*window] };
        // Windows are positioned by position_window().
        this.setShouldCascadeWindows(false);
        this.build_content(mtm, &window);
        position_window(&window);
        this.apply_native_colors();
        this.load_page();
        this
    }

    /// Viewer controllers of all open documents.
    pub fn all(mtm: MainThreadMarker) -> Vec<Retained<ViewerController>> {
        Document::all(mtm)
            .iter()
            .filter_map(|d| d.viewer())
            .collect()
    }

    fn document(&self) -> Option<Retained<Document>> {
        self.ivars().document.load()
    }

    pub fn web_view(&self) -> &WKWebView {
        self.ivars()
            .web_view
            .get()
            .expect("web view is created in new()")
    }

    fn find_bar(&self) -> &FindBar {
        self.ivars()
            .find_bar
            .get()
            .expect("find bar is created in new()")
    }

    fn build_content(&self, mtm: MainThreadMarker, window: &NSWindow) {
        let web_view = self.make_web_view(mtm);
        let content = NSView::new(mtm);
        let find_bar = FindBar::builder(mtm, self).install(&content, &web_view);
        window.setContentView(Some(&content));
        window.setInitialFirstResponder(Some(&web_view));
        let _ = self.ivars().web_view.set(web_view);
        let _ = self.ivars().find_bar.set(find_bar);
    }

    fn make_web_view(&self, mtm: MainThreadMarker) -> Retained<WKWebView> {
        let configuration = unsafe { WKWebViewConfiguration::new(mtm) };
        let handler = SchemeHandler::new(mtm, self);
        unsafe {
            configuration.setURLSchemeHandler_forURLScheme(
                Some(ProtocolObject::from_ref(&*handler)),
                &ns(smv_core::url::SCHEME),
            );
            configuration
                .defaultWebpagePreferences()
                .setAllowsContentJavaScript(false);
            // Code blocks and tables keep their (light) backgrounds on paper.
            configuration.preferences().setShouldPrintBackgrounds(true);
        }
        let world = unsafe { WKContentWorld::worldWithName(&ns(CONTENT_WORLD), mtm) };
        let script = unsafe {
            WKUserScript::initWithSource_injectionTime_forMainFrameOnly_inContentWorld(
                WKUserScript::alloc(mtm),
                &ns(VIEWER_JS),
                WKUserScriptInjectionTime::AtDocumentEnd,
                true,
                &world,
            )
        };
        unsafe { configuration.userContentController().addUserScript(&script) };
        let _ = self.ivars().world.set(world);

        let web_view = unsafe {
            WKWebView::initWithFrame_configuration(
                WKWebView::alloc(mtm),
                NSRect::ZERO,
                &configuration,
            )
        };
        unsafe {
            web_view.setNavigationDelegate(Some(ProtocolObject::from_ref(self)));
            web_view.setPageZoom(settings_store::current().zoom);
            if cfg!(debug_assertions) {
                web_view.setInspectable(true);
            }
        }
        web_view
    }

    // --- Loading and updating the page -------------------------------------

    fn load_page(&self) {
        let Some(path) = self.document().and_then(|d| d.path()) else {
            return;
        };
        self.ivars().loaded.set(false);
        let url_string = smv_core::url::path_to_url(&path.to_string_lossy());
        if let Some(url) = NSURL::URLWithString(&ns(&url_string)) {
            unsafe {
                self.web_view()
                    .loadRequest(&NSURLRequest::requestWithURL(&url))
            };
        }
    }

    /// The complete page for `path` when it is this window's document.
    pub fn page_for_path(&self, path: &str) -> Option<String> {
        let document = self.document()?;
        if document.path()?.to_str()? != path {
            return None;
        }
        let content = smv_core::markdown::render(&document.source());
        let css = settings_store::user_stylesheet(self.mtm());
        let ivars = self.ivars();
        ivars.page_content_version.set(ivars.content_version.get());
        ivars.page_style_version.set(ivars.style_version.get());
        Some(smv_core::page::page(&content, &css))
    }

    fn is_document_path(&self, path: &str) -> bool {
        self.document()
            .and_then(|d| d.path())
            .is_some_and(|p| p.to_str() == Some(path))
    }

    fn page_did_load(&self) {
        let ivars = self.ivars();
        ivars.loaded.set(true);
        if ivars.content_version.get() != ivars.page_content_version.get() {
            self.push_content();
        }
        if ivars.style_version.get() != ivars.page_style_version.get() {
            self.push_style();
        }
        if let Some(y) = ivars.pending_scroll.take() {
            self.call_js(
                "smv.restoreScroll(y);",
                &[("y", &NSNumber::new_f64(y))],
                None,
            );
        }
        let fragment = ivars.pending_fragment.take();
        if let Some(fragment) = fragment {
            self.scroll_to_fragment(&fragment);
        }
        if self.find_bar().is_visible() {
            self.run_find();
        }
        crate::diagnostics::page_loaded(self);
    }

    /// Called by the document after its text changed.
    pub fn content_changed(&self) {
        let ivars = self.ivars();
        ivars.content_version.set(ivars.content_version.get() + 1);
        if ivars.loaded.get() {
            self.push_content();
        }
    }

    fn push_content(&self) {
        let Some(document) = self.document() else {
            return;
        };
        let html = smv_core::markdown::render(&document.source());
        let ivars = self.ivars();
        ivars.page_content_version.set(ivars.content_version.get());
        let weak = Weak::from(self);
        self.call_js(
            "return smv.update(html);",
            &[("html", &ns(&html))],
            Some(Box::new(move |result| {
                if let Some(this) = weak.load() {
                    this.show_find_result(result);
                }
            })),
        );
    }

    /// Applies the current settings to this window (STANDARD §9.1).
    pub fn apply_settings(&self) {
        unsafe { self.web_view().setPageZoom(settings_store::current().zoom) };
        self.apply_native_colors();
        let ivars = self.ivars();
        ivars.style_version.set(ivars.style_version.get() + 1);
        if ivars.loaded.get() {
            self.push_style();
        }
    }

    fn push_style(&self) {
        let css = settings_store::user_stylesheet(self.mtm());
        let ivars = self.ivars();
        ivars.page_style_version.set(ivars.style_version.get());
        self.call_js("smv.setStyle(css);", &[("css", &ns(&css))], None);
    }

    fn apply_native_colors(&self) {
        let settings = settings_store::current();
        let background = match settings.color_mode {
            ColorMode::Automatic => NSColor::textBackgroundColor(),
            ColorMode::Custom => util::color(settings.background_color),
        };
        if let Some(window) = self.window() {
            window.setBackgroundColor(Some(&background));
        }
        unsafe {
            self.web_view()
                .setUnderPageBackgroundColor(Some(&background))
        };
    }

    fn reload_page_keeping_scroll(&self) {
        let weak = Weak::from(self);
        self.call_js(
            "return smv.scrollPosition();",
            &[],
            Some(Box::new(move |result| {
                if let Some(this) = weak.load() {
                    this.ivars().pending_scroll.set(number_value(result));
                    this.load_page();
                }
            })),
        );
        if !self.ivars().loaded.get() {
            self.load_page();
        }
    }

    /// Called by the document after its file was moved or renamed.
    pub fn document_moved(&self) {
        self.reload_page_keeping_scroll();
    }

    pub fn scroll_to_fragment(&self, fragment: &str) {
        if self.ivars().loaded.get() {
            self.call_js(
                "return smv.scrollToFragment(fragment);",
                &[("fragment", &ns(fragment))],
                None,
            );
        } else {
            self.ivars()
                .pending_fragment
                .replace(Some(fragment.to_owned()));
        }
    }

    /// Calls a function body in the app's content world of the page.
    fn call_js(
        &self,
        body: &str,
        arguments: &[(&str, &AnyObject)],
        completion: Option<JsCompletion>,
    ) {
        let keys: Vec<Retained<NSString>> = arguments.iter().map(|(k, _)| ns(k)).collect();
        let keys: Vec<&NSString> = keys.iter().map(|k| &**k).collect();
        let values: Vec<&AnyObject> = arguments.iter().map(|(_, v)| *v).collect();
        let arguments = NSDictionary::from_slices(&keys, &values);
        let block = completion.map(|completion| {
            RcBlock::new(move |result: *mut AnyObject, _error: *mut NSError| {
                completion(unsafe { result.as_ref() });
            })
        });
        let world = self.ivars().world.get().expect("world is created in new()");
        unsafe {
            self.web_view()
                .callAsyncJavaScript_arguments_inFrame_inContentWorld_completionHandler(
                    &ns(body),
                    Some(&arguments),
                    None,
                    world,
                    block.as_deref(),
                );
        }
    }

    /// Evaluates a diagnostic script and passes its JSON-encoded result on.
    pub fn evaluate_for_diagnostics(&self, script: &str, completion: impl Fn(String) + 'static) {
        let body = format!("return JSON.stringify(await (async () => {{ {script} }})());");
        self.call_js(
            &body,
            &[],
            Some(Box::new(move |result| {
                completion(string_value(result).unwrap_or_else(|| "null".to_owned()));
            })),
        );
    }

    // --- Links (STANDARD §4.3) ---------------------------------------------

    fn navigation_policy(&self, action: &WKNavigationAction) -> WKNavigationActionPolicy {
        let request = unsafe { action.request() };
        let Some(url) = request.URL() else {
            return WKNavigationActionPolicy::Cancel;
        };
        let main_frame = unsafe { action.targetFrame() }.is_none_or(|f| unsafe { f.isMainFrame() });
        if !main_frame {
            return WKNavigationActionPolicy::Allow;
        }
        let scheme = url
            .scheme()
            .map(|s| s.to_string().to_ascii_lowercase())
            .unwrap_or_default();
        let link = unsafe { action.navigationType() } == WKNavigationType::LinkActivated;
        match scheme.as_str() {
            "smv" | "file" => {
                let Some(path) = util::url_path(&url) else {
                    return WKNavigationActionPolicy::Cancel;
                };
                if scheme == "smv" && self.is_document_path(&path) {
                    return WKNavigationActionPolicy::Allow;
                }
                if scheme == "file" || link {
                    let fragment = url.fragment().map(|f| f.to_string());
                    open_local(self.mtm(), &path, fragment);
                }
                WKNavigationActionPolicy::Cancel
            }
            "about" => WKNavigationActionPolicy::Allow,
            "javascript" | "data" | "blob" => WKNavigationActionPolicy::Cancel,
            _ => {
                if link {
                    NSWorkspace::sharedWorkspace().openURL(&url);
                }
                WKNavigationActionPolicy::Cancel
            }
        }
    }

    // --- Margins, zoom and menu validation ---------------------------------

    /// Width of the page in CSS pixels.
    fn viewport_width(&self) -> f64 {
        let zoom = settings_store::current().zoom;
        self.web_view().bounds().size.width / zoom
    }

    fn validate(&self, item: &NSMenuItem) -> bool {
        let settings = settings_store::current();
        let Some(action) = item.action() else {
            return true;
        };
        if action == sel!(smvZoomIn:) {
            settings.can_zoom_in()
        } else if action == sel!(smvZoomOut:) {
            settings.can_zoom_out()
        } else if action == sel!(smvActualSize:) {
            (settings.zoom - 1.0).abs() > 1e-6
        } else if action == sel!(smvAutomaticMargins:) {
            let automatic = settings.margin_mode == MarginMode::Automatic;
            item.setState(if automatic {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
            true
        } else if action == sel!(smvFindNext:) || action == sel!(smvFindPrevious:) {
            !self.ivars().find_query.borrow().is_empty()
        } else {
            true
        }
    }

    // --- Search (STANDARD §6) -----------------------------------------------

    fn show_find(&self, focus: bool) {
        let find_bar = self.find_bar();
        find_bar.show();
        if focus && let Some(window) = self.window() {
            window.makeFirstResponder(Some(find_bar.field()));
            unsafe { find_bar.field().selectText(None) };
        }
    }

    fn close_find(&self) {
        self.find_bar().hide();
        self.call_js("return smv.finish();", &[], None);
        if let Some(window) = self.window() {
            window.makeFirstResponder(Some(self.web_view()));
        }
    }

    fn run_find(&self) {
        let query = self.find_bar().query();
        self.ivars().find_query.replace(query.clone());
        if query.is_empty() {
            self.call_js("return smv.clear();", &[], None);
            self.find_bar().set_status("");
            return;
        }
        let case_sensitive = NSNumber::new_bool(self.find_bar().case_sensitive());
        let weak = Weak::from(self);
        self.call_js(
            "return smv.find(query, caseSensitive);",
            &[("query", &ns(&query)), ("caseSensitive", &case_sensitive)],
            Some(Box::new(move |result| {
                if let Some(this) = weak.load() {
                    this.show_find_result(result);
                }
            })),
        );
    }

    fn step_find(&self, backwards: bool) {
        if self.find_bar().query() != *self.ivars().find_query.borrow() {
            return self.run_find();
        }
        let weak = Weak::from(self);
        self.call_js(
            "return smv.step(backwards);",
            &[("backwards", &NSNumber::new_bool(backwards))],
            Some(Box::new(move |result| {
                if let Some(this) = weak.load() {
                    this.show_find_result(result);
                }
            })),
        );
    }

    /// Find Next / Find Previous from the menu.
    fn find_again(&self, backwards: bool) {
        if !self.find_bar().is_visible() {
            self.show_find(false);
            self.run_find();
        } else {
            self.step_find(backwards);
        }
    }

    fn show_find_result(&self, result: Option<&AnyObject>) {
        if self.find_bar().query().is_empty() {
            return self.find_bar().set_status("");
        }
        let Some(state) = result.and_then(|r| r.downcast_ref::<NSDictionary>()) else {
            return;
        };
        let field = |key: &str| {
            state
                .objectForKey(&ns(key))
                .and_then(|v| v.downcast::<NSNumber>().ok())
                .map(|n| n.integerValue())
                .unwrap_or_default()
        };
        let (count, index, capped) = (field("count"), field("index"), field("capped") != 0);
        let status = match count {
            0 => "Not found".to_owned(),
            _ => format!(
                "{} of {}{}",
                index + 1,
                count,
                if capped { "+" } else { "" }
            ),
        };
        self.find_bar().set_status(&status);
    }

    // --- Diagnostics ---------------------------------------------------------

    pub fn find_for_diagnostics(&self, query: &str) {
        self.find_bar().set_query(query);
        self.show_find(false);
        self.run_find();
    }

    pub fn document_for_diagnostics(&self) -> Option<Retained<Document>> {
        self.document()
    }

    pub fn find_status(&self) -> String {
        self.find_bar().status()
    }

    /// PNG image of the window content: the native views composited with a
    /// snapshot of the web view (which draws out of process).
    pub fn snapshot_png(&self, completion: impl Fn(Option<Vec<u8>>) + 'static) {
        let Some(content) = self.window().and_then(|w| w.contentView()) else {
            return completion(None);
        };
        let web_view = self.web_view().retain();
        let block = RcBlock::new(move |image: *mut NSImage, _error: *mut NSError| {
            let overlay = unsafe { image.as_ref() }.map(|image| (image, web_view.frame()));
            completion(crate::diagnostics::render_png(&content, overlay));
        });
        unsafe {
            self.web_view()
                .takeSnapshotWithConfiguration_completionHandler(None, &block)
        };
    }

    // --- Printing (STANDARD §4.5) -------------------------------------------

    pub fn print_operation(
        &self,
        print_info: &NSPrintInfo,
        settings: &NSDictionary<NSString, AnyObject>,
    ) -> Option<Retained<NSPrintOperation>> {
        let print_info = print_info.copy();
        unsafe { print_info.dictionary().addEntriesFromDictionary(settings) };
        print_info.setHorizontalPagination(NSPrintingPaginationMode::Fit);
        print_info.setVerticalPagination(NSPrintingPaginationMode::Automatic);
        print_info.setHorizontallyCentered(false);
        print_info.setVerticallyCentered(false);
        let operation = unsafe { self.web_view().printOperationWithPrintInfo(&print_info) };
        operation.setShowsPrintPanel(true);
        operation.setShowsProgressPanel(true);
        Some(operation)
    }
}

fn make_window(mtm: MainThreadMarker) -> Retained<NSWindow> {
    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(860.0, 780.0));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setMinSize(NSSize::new(320.0, 200.0));
    window.setTabbingIdentifier(&ns(TABBING_IDENTIFIER));
    window
}

/// Restores the saved frame for the first window and cascades the others.
fn position_window(window: &NSWindow) {
    let name = ns(FRAME_AUTOSAVE_NAME);
    if !window.setFrameUsingName(&name) {
        window.center();
    }
    let start = if window.setFrameAutosaveName(&name) {
        NSPoint::new(0.0, 0.0)
    } else {
        CASCADE_POINT.get()
    };
    CASCADE_POINT.set(window.cascadeTopLeftFromPoint(start));
}

/// Opens a local link target: Markdown files in the viewer, everything else
/// with its default application (STANDARD §4.3).
fn open_local(mtm: MainThreadMarker, path: &str, fragment: Option<String>) {
    let Ok(metadata) = std::fs::metadata(path) else {
        NSBeep();
        return;
    };
    let url = util::file_url(path);
    if !(metadata.is_file() && util::is_markdown_path(path)) {
        NSWorkspace::sharedWorkspace().openURL(&url);
        return;
    }
    let completion = RcBlock::new(
        move |document: *mut NSDocument, _already_open: Bool, _error: *mut NSError| {
            let Some(fragment) = &fragment else {
                return;
            };
            let viewer = unsafe { document.as_ref() }
                .and_then(|d| d.downcast_ref::<Document>())
                .and_then(|d| d.viewer());
            if let Some(viewer) = viewer {
                viewer.scroll_to_fragment(fragment);
            }
        },
    );
    NSDocumentController::sharedDocumentController(mtm)
        .openDocumentWithContentsOfURL_display_completionHandler(&url, true, &completion);
}

fn number_value(value: Option<&AnyObject>) -> Option<f64> {
    value
        .and_then(|v| v.downcast_ref::<NSNumber>())
        .map(|n| n.doubleValue())
}

fn string_value(value: Option<&AnyObject>) -> Option<String> {
    value
        .and_then(|v| v.downcast_ref::<NSString>())
        .map(|s| s.to_string())
}
