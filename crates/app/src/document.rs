//! The Markdown document: an `NSDocument` that only reads (STANDARD §4).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use dispatch2::{DispatchQueue, DispatchTime};
use objc2::rc::{Allocated, Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSDocument, NSDocumentController, NSPrintOperation};
use objc2_foundation::{NSData, NSDictionary, NSError, NSObject, NSString, NSURL};

use crate::viewer::ViewerController;
use crate::watcher::FileWatcher;

/// Changes reported within this interval are handled as one reload.
const RELOAD_DELAY: Duration = Duration::from_millis(100);

type FileStamp = (Option<SystemTime>, u64);

#[derive(Default)]
pub struct DocumentIvars {
    token: Cell<u64>,
    source: RefCell<String>,
    stamp: Cell<Option<FileStamp>>,
    watcher: RefCell<Option<FileWatcher>>,
    watched_path: RefCell<Option<PathBuf>>,
    reload_scheduled: Cell<bool>,
}

thread_local! {
    static DOCUMENTS: RefCell<HashMap<u64, Weak<Document>>> = RefCell::new(HashMap::new());
    static NEXT_TOKEN: Cell<u64> = const { Cell::new(1) };
}

define_class!(
    // SAFETY: NSDocument may be subclassed; the overridden methods call
    // super where AppKit expects it. No Drop impl.
    #[unsafe(super(NSDocument, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SMVDocument"]
    #[ivars = DocumentIvars]
    pub struct Document;

    impl Document {
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Option<Retained<Self>> {
            let this = this.set_ivars(DocumentIvars::default());
            let this: Option<Retained<Self>> = unsafe { msg_send![super(this), init] };
            if let Some(document) = &this {
                document.register();
            }
            this
        }

        #[unsafe(method(readFromData:ofType:error:))]
        fn read_from_data(
            &self,
            data: &NSData,
            _type_name: &NSString,
            _error: *mut *mut NSError,
        ) -> bool {
            self.set_source(smv_core::text::decode(&data.to_vec()));
            true
        }

        #[unsafe(method(makeWindowControllers))]
        fn make_window_controllers(&self) {
            let controller = ViewerController::new(self.mtm(), self);
            self.addWindowController(&controller);
            self.start_watching();
        }

        #[unsafe(method(setFileURL:))]
        fn set_file_url(&self, url: Option<&NSURL>) {
            let _: () = unsafe { msg_send![super(self), setFileURL: url] };
            if self.ivars().watched_path.borrow().is_some() {
                self.start_watching();
                if let Some(viewer) = self.viewer() {
                    viewer.document_moved();
                }
            }
        }

        #[unsafe(method_id(printOperationWithSettings:error:))]
        fn print_operation_with_settings(
            &self,
            settings: &NSDictionary<NSString, AnyObject>,
            _error: *mut *mut NSError,
        ) -> Option<Retained<NSPrintOperation>> {
            self.viewer()
                .and_then(|viewer| viewer.print_operation(&self.printInfo(), settings))
        }

        #[unsafe(method(close))]
        fn close(&self) {
            self.ivars().watcher.replace(None);
            self.ivars().watched_path.replace(None);
            DOCUMENTS.with(|docs| docs.borrow_mut().remove(&self.ivars().token.get()));
            let _: () = unsafe { msg_send![super(self), close] };
        }
    }
);

impl Document {
    fn register(&self) {
        let token = NEXT_TOKEN.with(|next| next.replace(next.get() + 1));
        self.ivars().token.set(token);
        DOCUMENTS.with(|docs| docs.borrow_mut().insert(token, Weak::from(self)));
    }

    fn with_token(token: u64) -> Option<Retained<Document>> {
        DOCUMENTS.with(|docs| docs.borrow().get(&token).and_then(Weak::load))
    }

    /// All open documents.
    pub fn all(mtm: MainThreadMarker) -> Vec<Retained<Document>> {
        NSDocumentController::sharedDocumentController(mtm)
            .documents()
            .iter()
            .filter_map(|document| document.downcast::<Document>().ok())
            .collect()
    }

    pub fn source(&self) -> String {
        self.ivars().source.borrow().clone()
    }

    pub fn path(&self) -> Option<PathBuf> {
        self.fileURL()?.path().map(|p| PathBuf::from(p.to_string()))
    }

    pub fn viewer(&self) -> Option<Retained<ViewerController>> {
        self.windowControllers()
            .iter()
            .find_map(|controller| controller.downcast::<ViewerController>().ok())
    }

    fn set_source(&self, text: String) {
        if *self.ivars().source.borrow() == text {
            return;
        }
        self.ivars().source.replace(text);
        if let Some(viewer) = self.viewer() {
            viewer.content_changed();
        }
    }

    /// Reads the file again. Unless `force` is set, the file is only read
    /// when its modification date or size changed. A missing file keeps the
    /// current content (STANDARD §4.4).
    pub fn reload_from_disk(&self, force: bool) {
        let Some(path) = self.path() else {
            return;
        };
        let Some(stamp) = stamp_of(&path) else {
            return;
        };
        if !force && self.ivars().stamp.get() == Some(stamp) {
            return;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        self.ivars().stamp.set(Some(stamp));
        self.set_source(smv_core::text::decode(&bytes));
    }

    fn start_watching(&self) {
        let Some(path) = self.path() else {
            return;
        };
        if self.ivars().watched_path.borrow().as_ref() == Some(&path) {
            return;
        }
        self.ivars().stamp.set(stamp_of(&path));
        let watcher = FileWatcher::new(&path, self.ivars().token.get(), file_event);
        self.ivars().watcher.replace(watcher);
        self.ivars().watched_path.replace(Some(path));
    }
}

fn stamp_of(path: &Path) -> Option<FileStamp> {
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.modified().ok(), metadata.len()))
}

/// Runs on the main thread for every change in a document's directory.
fn file_event(token: u64) {
    let Some(document) = Document::with_token(token) else {
        return;
    };
    if document.ivars().reload_scheduled.replace(true) {
        return;
    }
    let when = DispatchTime::try_from(RELOAD_DELAY).unwrap_or(DispatchTime::NOW);
    let _ = DispatchQueue::main().after(when, move || {
        if let Some(document) = Document::with_token(token) {
            document.ivars().reload_scheduled.set(false);
            document.reload_from_disk(false);
        }
    });
}
