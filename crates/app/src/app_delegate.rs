//! Application delegate: launch behavior, Settings, Open Recent and Help
//! (STANDARD §4.1, §9.2).

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationDelegate, NSDocumentController, NSMenu, NSMenuDelegate, NSMenuItem,
    NSWorkspace,
};
use objc2_foundation::{NSNotification, NSObject, NSURL};

use crate::settings_window::SettingsController;
use crate::util::{self, ns};
use crate::{diagnostics, menu};

const HELP_URL: &str = "https://github.com/skonmeme/skons_markdown_viewer#readme";

thread_local! {
    /// Set when documents were requested on the command line, so that no
    /// Open panel is shown while they are being opened.
    static OPENED_FROM_COMMAND_LINE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements; no Drop impl.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SMVAppDelegate"]
    pub struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationWillFinishLaunching:))]
        fn will_finish_launching(&self, _notification: &NSNotification) {
            let mtm = self.mtm();
            NSApplication::sharedApplication(mtm).setMainMenu(Some(&menu::build(mtm, self)));
            open_command_line_documents(mtm);
        }

        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            diagnostics::app_launched(self.mtm());
        }

        #[unsafe(method(applicationShouldOpenUntitledFile:))]
        fn should_open_untitled_file(&self, _app: &NSApplication) -> bool {
            !diagnostics::is_active() && !OPENED_FROM_COMMAND_LINE.get()
        }

        /// There are no untitled documents; launching or reopening the app
        /// without a document shows the Open panel instead (STANDARD §4.1).
        #[unsafe(method(applicationOpenUntitledFile:))]
        fn open_untitled_file(&self, _app: &NSApplication) -> bool {
            unsafe { NSDocumentController::sharedDocumentController(self.mtm()).openDocument(None) };
            true
        }

        #[unsafe(method(applicationSupportsSecureRestorableState:))]
        fn supports_secure_restorable_state(&self, _app: &NSApplication) -> bool {
            true
        }
    }

    unsafe impl NSMenuDelegate for AppDelegate {
        #[unsafe(method(menuNeedsUpdate:))]
        fn menu_needs_update(&self, menu: &NSMenu) {
            menu::fill_open_recent(self.mtm(), menu, self);
        }
    }

    impl AppDelegate {
        #[unsafe(method(smvShowSettings:))]
        fn show_settings(&self, _sender: Option<&AnyObject>) {
            SettingsController::show(self.mtm());
        }

        #[unsafe(method(smvOpenRecent:))]
        fn open_recent(&self, sender: &NSMenuItem) {
            let Some(url) = sender.representedObject().and_then(|o| o.downcast::<NSURL>().ok()) else {
                return;
            };
            open_document(self.mtm(), &url);
        }

        #[unsafe(method(smvShowHelp:))]
        fn show_help(&self, _sender: Option<&AnyObject>) {
            if let Some(url) = NSURL::URLWithString(&ns(HELP_URL)) {
                NSWorkspace::sharedWorkspace().openURL(&url);
            }
        }
    }
);

impl AppDelegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

fn open_document(mtm: MainThreadMarker, url: &NSURL) {
    let completion = block2::RcBlock::new(
        |_document: *mut objc2_app_kit::NSDocument,
         _already_open: objc2::runtime::Bool,
         _error: *mut objc2_foundation::NSError| {},
    );
    NSDocumentController::sharedDocumentController(mtm)
        .openDocumentWithContentsOfURL_display_completionHandler(url, true, &completion);
}

/// Opens Markdown files passed as command line arguments.
fn open_command_line_documents(mtm: MainThreadMarker) {
    let current_dir = std::env::current_dir().unwrap_or_default();
    for argument in std::env::args().skip(1) {
        if argument.starts_with('-') {
            continue;
        }
        let path = current_dir.join(&argument);
        if path.is_file() {
            OPENED_FROM_COMMAND_LINE.set(true);
            open_document(mtm, &util::file_url(&path.to_string_lossy()));
        }
    }
}
