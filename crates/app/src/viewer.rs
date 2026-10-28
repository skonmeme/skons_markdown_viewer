//! A document window: the web view showing the rendered document, the find
//! bar and the bridge to the page script (STANDARD §4, §5 and §6).

use std::cell::{Cell, OnceCell, RefCell};

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, Bool, NSObjectProtocol, ProtocolObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSBeep, NSColor, NSControl, NSControlStateValueOff,
    NSControlStateValueOn, NSControlTextEditingDelegate, NSDocument, NSDocumentController,
    NSEventModifierFlags, NSMenuItem, NSMenuItemValidation, NSPrintInfo, NSPrintOperation,
    NSPrintingPaginationMode, NSResponder, NSSearchFieldDelegate, NSTextFieldDelegate, NSTextView,
    NSView, NSWindow, NSWindowController, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{
    NSCopying, NSDictionary, NSError, NSNotification, NSNumber, NSObject, NSPoint, NSRange, NSRect,
    NSSize, NSString, NSURL, NSURLRequest,
};
use objc2_web_kit::{
    WKContentWorld, WKNavigation, WKNavigationAction, WKNavigationActionPolicy,
    WKNavigationDelegate, WKNavigationType, WKUserScript, WKUserScriptInjectionTime, WKWebView,
    WKWebViewConfiguration,
};
use smv_core::ai::Block;
use smv_core::page::{CONTENT_WORLD, VIEWER_JS};
use smv_core::search::{self, Mode};
use smv_core::settings::{ColorMode, MarginMode};

use crate::ai_search::{self, BatchResult, Progress};
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
    /// Mode and query of the last search that was started with Return
    /// (STANDARD §6.2); `None` after the query changed.
    searched: RefCell<Option<(Mode, String)>>,
    /// The AI search of this window while it runs (STANDARD §6.6).
    ai: RefCell<Option<ai_search::Search>>,
    /// Raised whenever an AI search starts or stops, so that late answers
    /// are ignored.
    ai_generation: Cell<u64>,
    /// Set while a typed command turns into a token.
    converting: Cell<bool>,
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
        fn control_do_command(&self, _control: &NSControl, view: &NSTextView, command: Sel) -> bool {
            if command == sel!(insertNewline:) {
                let shift = NSApplication::sharedApplication(self.mtm())
                    .currentEvent()
                    .is_some_and(|e| e.modifierFlags().contains(NSEventModifierFlags::Shift));
                self.step_find(shift);
                true
            } else if command == sel!(cancelOperation:) {
                self.close_find();
                true
            } else if command == sel!(deleteBackward:) {
                // Delete right after the token removes it (STANDARD §6.3).
                let at_start = view.selectedRange() == NSRange::new(0, 0);
                let remove = at_start && self.find_bar().token().is_some();
                if remove {
                    self.find_bar().set_token(None, 0);
                    self.mode_changed();
                }
                remove
            } else {
                false
            }
        }

        /// Sent at every keystroke, unlike the action of the search field,
        /// which waits for a pause in typing.
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _notification: &NSNotification) {
            if self.convert_typed_command() {
                self.mode_changed();
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
                    // Searches at once in every mode and keeps the token (STANDARD §6.1).
                    this.find_bar().set_query(&text);
                    this.show_find(false);
                    if this.mode().starts_with_return() {
                        this.start_search();
                    } else {
                        this.run_find();
                    }
                })),
            );
        }

        #[unsafe(method(smvFindFieldChanged:))]
        fn find_field_changed_action(&self, _sender: Option<&AnyObject>) {
            self.find_field_changed();
        }

        #[unsafe(method(smvFindClear:))]
        fn find_clear_action(&self, _sender: Option<&AnyObject>) {
            // The clear button removes the token and the text (STANDARD §6.3).
            self.find_bar().set_query("");
            self.find_bar().set_token(None, 0);
            self.mode_changed();
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
            searched: RefCell::new(None),
            ai: RefCell::new(None),
            ai_generation: Cell::new(0),
            converting: Cell::new(false),
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
            self.repeat_search();
        }
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
                    // The page repeats keyword and regular expression
                    // searches; an AI search runs again (STANDARD §6.1).
                    if this.find_bar().is_visible() && this.mode() == Mode::Ai {
                        this.repeat_search();
                    }
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
        self.stop_ai();
        self.ivars().searched.replace(None);
        self.find_bar().hide();
        self.call_js("return smv.finish();", &[], None);
        if let Some(window) = self.window() {
            window.makeFirstResponder(Some(self.web_view()));
        }
    }

    /// The mode of the token, or keyword search without one (STANDARD §6.2).
    fn mode(&self) -> Mode {
        self.find_bar().token().unwrap_or(Mode::Keyword)
    }

    /// The action of the search field: its text changed.
    fn find_field_changed(&self) {
        if self.ivars().converting.get() {
            return;
        }
        if self.convert_typed_command() {
            return self.mode_changed();
        }
        if self.mode().starts_with_return() {
            self.query_edited();
        } else {
            self.run_find();
        }
    }

    /// Turns a command that was just typed at the start of the field, with
    /// its Space, into a token (STANDARD §6.3).
    fn convert_typed_command(&self) -> bool {
        let find_bar = self.find_bar();
        if self.ivars().converting.get() || find_bar.token().is_some() {
            return false;
        }
        let Some(caret) = find_bar.caret() else {
            return false;
        };
        let text = find_bar.query();
        let Some(command) = search::typed_command(&text, caret) else {
            return false;
        };
        self.ivars().converting.set(true);
        find_bar.set_query(&text[command.length..]);
        find_bar.set_token(Some(command.mode), 0);
        self.ivars().converting.set(false);
        true
    }

    /// A token was added or removed: results of the old mode no longer
    /// apply, and a keyword search runs at once.
    fn mode_changed(&self) {
        self.stop_ai();
        self.ivars().searched.replace(None);
        if self.mode().starts_with_return() {
            self.clear_results();
        } else {
            self.run_find();
        }
    }

    /// Editing the query of a mode that starts with Return stops a running
    /// search and removes its results.
    fn query_edited(&self) {
        let query = self.find_bar().query();
        let mode = self.mode();
        let current = self
            .ivars()
            .searched
            .borrow()
            .as_ref()
            .is_some_and(|(m, q)| *m == mode && *q == query);
        if !current {
            self.stop_ai();
            self.ivars().searched.replace(None);
            self.clear_results();
        }
    }

    fn clear_results(&self) {
        self.call_js("return smv.clear();", &[], None);
        self.find_bar().set_status("", None);
    }

    /// Keyword search, run while the user types (STANDARD §6.4).
    fn run_find(&self) {
        let query = self.find_bar().query();
        self.ivars().find_query.replace(query.clone());
        if query.is_empty() {
            return self.clear_results();
        }
        let case_sensitive = NSNumber::new_bool(self.find_bar().case_sensitive());
        self.js_find(&query, &case_sensitive, false);
    }

    fn js_find(&self, query: &str, case_sensitive: &NSNumber, regex: bool) {
        let weak = Weak::from(self);
        self.call_js(
            "return smv.find(query, caseSensitive, regex);",
            &[
                ("query", &ns(query)),
                ("caseSensitive", case_sensitive),
                ("regex", &NSNumber::new_bool(regex)),
            ],
            Some(Box::new(move |result| {
                if let Some(this) = weak.load() {
                    this.show_find_result(result);
                }
            })),
        );
    }

    /// Starts a regular expression or AI search (STANDARD §6.5, §6.6).
    fn start_search(&self) {
        let mode = self.mode();
        let query = self.find_bar().query();
        self.stop_ai();
        self.ivars().find_query.replace(query.clone());
        if query.is_empty() {
            self.ivars().searched.replace(None);
            return self.clear_results();
        }
        self.ivars().searched.replace(Some((mode, query.clone())));
        match mode {
            Mode::Ai => self.start_ai(query),
            Mode::RegexIgnoringCase | Mode::RegexMatchingCase => {
                let case_sensitive = NSNumber::new_bool(mode == Mode::RegexMatchingCase);
                self.js_find(&query, &case_sensitive, true);
            }
            Mode::Keyword => self.run_find(),
        }
    }

    /// Runs the current search again, for example after the document was
    /// reloaded (STANDARD §6.1).
    fn repeat_search(&self) {
        let mode = self.mode();
        if !mode.starts_with_return() {
            return self.run_find();
        }
        let query = self.find_bar().query();
        let searched = self
            .ivars()
            .searched
            .borrow()
            .as_ref()
            .is_some_and(|(m, q)| *m == mode && *q == query);
        if searched {
            self.start_search();
        }
    }

    /// Return, ⌘G and the Previous and Next buttons (STANDARD §6.1, §6.2).
    fn step_find(&self, backwards: bool) {
        let mode = self.mode();
        let query = self.find_bar().query();
        if mode.starts_with_return() {
            let searched = self
                .ivars()
                .searched
                .borrow()
                .as_ref()
                .is_some_and(|(m, q)| *m == mode && *q == query);
            if !searched {
                return self.start_search();
            }
        } else if query != *self.ivars().find_query.borrow() {
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
            if self.mode().starts_with_return() {
                self.step_find(backwards);
            } else {
                self.run_find();
            }
        } else {
            self.step_find(backwards);
        }
    }

    fn show_find_result(&self, result: Option<&AnyObject>) {
        let find_bar = self.find_bar();
        if find_bar.query().is_empty()
            || (self.mode().starts_with_return() && self.ivars().searched.borrow().is_none())
        {
            return find_bar.set_status("", None);
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
        if field("invalid") != 0 {
            return find_bar.set_status("Invalid pattern", None);
        }
        if let Some(search) = self.ivars().ai.borrow().as_ref() {
            let (done, total) = search.progress();
            return find_bar.set_status(&ai_search::searching_status(done, total), None);
        }
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
        find_bar.set_status(&status, None);
    }

    // --- AI search (STANDARD §6.6) ------------------------------------------

    fn start_ai(&self, query: String) {
        let generation = self.ivars().ai_generation.get();
        self.find_bar().set_status("Searching…", None);
        let weak = Weak::from(self);
        self.call_js(
            "return JSON.stringify(smv.aiBlocks(query));",
            &[("query", &ns(&query))],
            Some(Box::new(move |result| {
                let Some(this) = weak.load() else {
                    return;
                };
                if this.ivars().ai_generation.get() != generation {
                    return;
                }
                let blocks: Vec<Block> = string_value(result)
                    .and_then(|json| serde_json::from_str(&json).ok())
                    .unwrap_or_default();
                match ai_search::Search::new(&settings_store::current(), &query, &blocks) {
                    Ok(search) => {
                        this.ivars().ai.replace(Some(search));
                        this.send_next_batch();
                    }
                    Err(reason) => this.ai_unavailable(&reason),
                }
            })),
        );
    }

    fn send_next_batch(&self) {
        let generation = self.ivars().ai_generation.get();
        let weak = Weak::from(self);
        let sent = match self.ivars().ai.borrow_mut().as_mut() {
            Some(search) => search.send_next(move |result| {
                if let Some(this) = weak.load() {
                    this.batch_finished(generation, result);
                }
            }),
            None => return,
        };
        match sent {
            Ok(true) => self.show_ai_results(),
            Ok(false) => self.finish_ai(),
            Err(reason) => self.ai_unavailable(&reason),
        }
    }

    fn batch_finished(&self, generation: u64, result: BatchResult) {
        if self.ivars().ai_generation.get() != generation {
            return;
        }
        let progress = match self.ivars().ai.borrow_mut().as_mut() {
            Some(search) => search.record(result),
            None => return,
        };
        match progress {
            Progress::Searching => self.send_next_batch(),
            Progress::Finished => self.finish_ai(),
            Progress::Unavailable(reason) => self.ai_unavailable(&reason),
        }
    }

    /// Highlights the blocks chosen so far.
    fn show_ai_results(&self) {
        let chosen = match self.ivars().ai.borrow().as_ref() {
            Some(search) => search.chosen(),
            None => return,
        };
        self.show_blocks(&chosen);
    }

    fn finish_ai(&self) {
        let Some(search) = self.ivars().ai.take() else {
            return;
        };
        let chosen = search.chosen();
        drop(search);
        self.show_blocks(&chosen);
    }

    fn show_blocks(&self, chosen: &[usize]) {
        let indices = serde_json::to_string(chosen).expect("numbers are serializable");
        let weak = Weak::from(self);
        self.call_js(
            "return smv.showBlocks(JSON.parse(indices));",
            &[("indices", &ns(&indices))],
            Some(Box::new(move |result| {
                if let Some(this) = weak.load() {
                    this.show_find_result(result);
                }
            })),
        );
    }

    /// AI search cannot be used: nothing is highlighted and the counter
    /// tells why when pointed at.
    fn ai_unavailable(&self, reason: &str) {
        self.stop_ai();
        self.call_js("return smv.clear();", &[], None);
        self.find_bar().set_status("AI unavailable", Some(reason));
    }

    /// Stops the AI search of this window, if one runs.
    fn stop_ai(&self) {
        let ivars = self.ivars();
        ivars.ai_generation.set(ivars.ai_generation.get() + 1);
        drop(ivars.ai.take());
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
