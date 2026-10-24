//! Developer diagnostics: the snapshot mode.
//!
//! When one of the snapshot environment variables is set, the app renders
//! the requested window into a PNG file, prints a JSON line to standard
//! output and quits. Tests and the documentation screenshots use this.

use std::cell::Cell;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::Weak;
use objc2_app_kit::{
    NSAppearanceCustomization, NSApplication, NSBezierPath, NSBitmapImageFileType, NSColor,
    NSCompositingOperation, NSGraphicsContext, NSImage, NSPrintJobSavingURL, NSPrintSaveJob,
    NSView,
};
use objc2_foundation::{NSDictionary, NSRect, NSSize, NSTimer};

use crate::document::Document;
use crate::settings_window::SettingsController;
use crate::viewer::ViewerController;

/// PNG file for the first document window.
const SNAPSHOT: &str = "SMV_SNAPSHOT";
/// Content size `WIDTHxHEIGHT` (points) of the window before the snapshot.
const SNAPSHOT_SIZE: &str = "SMV_SNAPSHOT_SIZE";
/// Text searched with the find bar before the snapshot.
const SNAPSHOT_FIND: &str = "SMV_SNAPSHOT_FIND";
/// Script run in the page's `smv` content world; its result is printed.
const SNAPSHOT_EVAL: &str = "SMV_SNAPSHOT_EVAL";
/// PDF file the document is printed to after the snapshot.
const SNAPSHOT_PRINT: &str = "SMV_SNAPSHOT_PRINT";
/// PNG file for the Settings window.
const SNAPSHOT_SETTINGS: &str = "SMV_SNAPSHOT_SETTINGS";

/// Time given to a print job before the app quits.
const PRINT_SECONDS: f64 = 4.0;

/// Time given to fonts, images and layout to settle before capturing.
const SETTLE_SECONDS: f64 = 0.8;

thread_local! {
    static TAKEN: Cell<bool> = const { Cell::new(false) };
}

pub fn is_active() -> bool {
    std::env::var_os(SNAPSHOT).is_some() || std::env::var_os(SNAPSHOT_SETTINGS).is_some()
}

pub fn app_launched(mtm: MainThreadMarker) {
    let Ok(path) = std::env::var(SNAPSHOT_SETTINGS) else {
        return;
    };
    SettingsController::show(mtm);
    after(SETTLE_SECONDS, move || {
        let png = SettingsController::snapshot_png(mtm);
        finish(mtm, &path, png, None, None, None);
    });
}

pub fn page_loaded(viewer: &ViewerController) {
    let Ok(path) = std::env::var(SNAPSHOT) else {
        return;
    };
    if TAKEN.replace(true) {
        return;
    }
    let mtm = MainThreadMarker::new().expect("diagnostics run on the main thread");
    if let Some((width, height)) = std::env::var(SNAPSHOT_SIZE)
        .ok()
        .as_deref()
        .and_then(parse_size)
        && let Some(window) = viewer.window()
    {
        window.setContentSize(NSSize::new(width, height));
    }
    if let Ok(query) = std::env::var(SNAPSHOT_FIND) {
        viewer.find_for_diagnostics(&query);
    }
    let viewer = Weak::from(viewer);
    after(SETTLE_SECONDS, move || {
        let Some(viewer) = viewer.load() else {
            return;
        };
        let script = std::env::var(SNAPSHOT_EVAL).unwrap_or_else(|_| "return null;".to_owned());
        let capture = viewer.clone();
        let path = path.clone();
        viewer.evaluate_for_diagnostics(&script, move |eval| {
            let path = path.clone();
            let find = capture.find_status();
            let printer = capture.clone();
            capture.snapshot_png(move |png| {
                let Ok(pdf) = std::env::var(SNAPSHOT_PRINT) else {
                    return finish(mtm, &path, png, Some(&eval), Some(&find), None);
                };
                print_to_pdf(&printer, &pdf);
                let (path, eval, find) = (path.clone(), eval.clone(), find.clone());
                let png = std::cell::RefCell::new(png);
                after(PRINT_SECONDS, move || {
                    let png = png.borrow_mut().take();
                    finish(mtm, &path, png, Some(&eval), Some(&find), Some(&pdf));
                });
            });
        });
    });
}

fn finish(
    mtm: MainThreadMarker,
    path: &str,
    png: Option<Vec<u8>>,
    eval: Option<&str>,
    find: Option<&str>,
    pdf: Option<&str>,
) {
    let written = png.is_some_and(|png| std::fs::write(path, png).is_ok());
    let line = serde_json::json!({
        "snapshot": if written { Some(path) } else { None },
        "eval": eval.and_then(|e| serde_json::from_str::<serde_json::Value>(e).ok()),
        "find": find,
        "documents": Document::all(mtm).len(),
        "pdfBytes": pdf.and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len()),
    });
    println!("{line}");
    NSApplication::sharedApplication(mtm).terminate(None);
}

/// Prints the document into a PDF file without showing any panel.
fn print_to_pdf(viewer: &ViewerController, pdf: &str) {
    let (Some(window), Some(document)) = (viewer.window(), viewer.document_for_diagnostics())
    else {
        return;
    };
    let Some(operation) = viewer.print_operation(&document.printInfo(), &NSDictionary::new())
    else {
        return;
    };
    let info = operation.printInfo();
    unsafe {
        info.setJobDisposition(NSPrintSaveJob);
        info.dictionary()
            .insert(NSPrintJobSavingURL, &*crate::util::file_url(pdf));
    }
    operation.setShowsPrintPanel(false);
    operation.setShowsProgressPanel(false);
    unsafe {
        operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
            &window,
            None,
            None,
            std::ptr::null_mut(),
        );
    }
}

fn parse_size(text: &str) -> Option<(f64, f64)> {
    let (width, height) = text.split_once('x')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

/// Renders `content` into a PNG image on top of its window background.
/// `overlay` is drawn last, for content that draws out of process (the web
/// view), given with its frame in `content` coordinates.
pub fn render_png(content: &NSView, overlay: Option<(&NSImage, NSRect)>) -> Option<Vec<u8>> {
    let bounds = content.bounds();
    let views = content.bitmapImageRepForCachingDisplayInRect(bounds)?;
    content.cacheDisplayInRect_toBitmapImageRep(bounds, &views);
    let output = content.bitmapImageRepForCachingDisplayInRect(bounds)?;
    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&output)?;
    let background = content
        .window()
        .map_or_else(NSColor::windowBackgroundColor, |w| w.backgroundColor());
    let draw = RcBlock::new(move || {
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&context));
        background.setFill();
        NSBezierPath::fillRect(bounds);
        // Image reps draw with "copy" by default; blend over the background.
        unsafe {
            views.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                bounds,
                NSRect::ZERO,
                NSCompositingOperation::SourceOver,
                1.0,
                false,
                None,
            )
        };
        if let Some((image, frame)) = overlay {
            image.drawInRect(frame);
        }
        context.flushGraphics();
        NSGraphicsContext::restoreGraphicsState_class();
    });
    // Resolve dynamic system colors with the window's (light or dark) look.
    content
        .effectiveAppearance()
        .performAsCurrentDrawingAppearance(&draw);
    let data = unsafe {
        output.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }?;
    Some(data.to_vec())
}

/// Runs `action` once on the main run loop after `seconds`.
pub fn after(seconds: f64, action: impl Fn() + 'static) {
    let block = RcBlock::new(move |_timer: NonNull<NSTimer>| action());
    unsafe { NSTimer::scheduledTimerWithTimeInterval_repeats_block(seconds, false, &block) };
}
