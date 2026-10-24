//! skon's Markdown Viewer — macOS application entry point.
//!
//! The app is a document-based AppKit application written in Rust through the
//! objc2 bindings. Each Markdown file is an `NSDocument` shown in its own
//! window with a `WKWebView` (see doc/ARCHITECTURE.md).

mod app_delegate;
mod diagnostics;
mod document;
mod find_bar;
mod fonts;
mod menu;
mod scheme;
mod settings_store;
mod settings_window;
mod util;
mod viewer;
mod watcher;

use objc2::runtime::ProtocolObject;
use objc2::{ClassType, MainThreadMarker};
use objc2_app_kit::NSApplication;

fn main() {
    let mtm = MainThreadMarker::new().expect("the app must start on the main thread");

    // NSDocumentController looks the document class up by the name given in
    // Info.plist, so it has to be registered before the app starts.
    let _ = document::Document::class();

    let app = NSApplication::sharedApplication(mtm);
    let delegate = app_delegate::AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
}
