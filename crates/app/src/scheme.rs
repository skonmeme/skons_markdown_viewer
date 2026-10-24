//! Handler of the `smv:` URL scheme (STANDARD §4.2).
//!
//! The document itself is served as the rendered page; every other path is
//! served from the file system so that relative images and media work.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use objc2::rc::{Retained, Weak};
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{
    NSData, NSDictionary, NSError, NSHTTPURLResponse, NSString, NSURL, NSURLErrorDomain,
};
use objc2_uniform_type_identifiers::UTType;
use objc2_web_kit::{WKURLSchemeHandler, WKURLSchemeTask, WKWebView};
use smv_core::url::parse_byte_range;

use crate::util::ns;
use crate::viewer::ViewerController;

/// `NSURLErrorFileDoesNotExist`.
const FILE_DOES_NOT_EXIST: isize = -1100;

pub struct SchemeIvars {
    viewer: Weak<ViewerController>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements; no Drop impl.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SMVSchemeHandler"]
    #[ivars = SchemeIvars]
    pub struct SchemeHandler;

    unsafe impl NSObjectProtocol for SchemeHandler {}

    unsafe impl WKURLSchemeHandler for SchemeHandler {
        #[unsafe(method(webView:startURLSchemeTask:))]
        fn start_task(&self, _web_view: &WKWebView, task: &ProtocolObject<dyn WKURLSchemeTask>) {
            self.serve(task);
        }

        #[unsafe(method(webView:stopURLSchemeTask:))]
        fn stop_task(&self, _web_view: &WKWebView, _task: &ProtocolObject<dyn WKURLSchemeTask>) {
            // Tasks are answered synchronously, so there is nothing to cancel.
        }
    }
);

impl SchemeHandler {
    pub fn new(mtm: MainThreadMarker, viewer: &ViewerController) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SchemeIvars {
            viewer: Weak::from(viewer),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn serve(&self, task: &ProtocolObject<dyn WKURLSchemeTask>) {
        let request = unsafe { task.request() };
        let Some(url) = request.URL() else {
            return fail(task);
        };
        let Some(path) = url.path().map(|p| p.to_string()) else {
            return fail(task);
        };
        if let Some(page) = self
            .ivars()
            .viewer
            .load()
            .and_then(|v| v.page_for_path(&path))
        {
            return respond(
                task,
                &url,
                200,
                "text/html; charset=utf-8",
                &[],
                page.as_bytes(),
            );
        }
        let range = request
            .valueForHTTPHeaderField(&ns("Range"))
            .map(|r| r.to_string());
        serve_file(task, &url, &path, range.as_deref());
    }
}

fn serve_file(
    task: &ProtocolObject<dyn WKURLSchemeTask>,
    url: &NSURL,
    path: &str,
    range: Option<&str>,
) {
    let Ok(mut file) = File::open(path) else {
        return fail(task);
    };
    let length = match file.metadata() {
        Ok(metadata) if metadata.is_file() => metadata.len(),
        _ => return fail(task),
    };
    let mime = mime_type(path);
    let Some(range) = range else {
        let mut body = Vec::with_capacity(length as usize);
        if file.read_to_end(&mut body).is_err() {
            return fail(task);
        }
        return respond(task, url, 200, &mime, &[], &body);
    };
    let Some((start, end)) = parse_byte_range(range, length) else {
        let content_range = format!("bytes */{length}");
        return respond(
            task,
            url,
            416,
            &mime,
            &[("Content-Range", content_range)],
            &[],
        );
    };
    let mut body = vec![0; (end - start + 1) as usize];
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut body).is_err() {
        return fail(task);
    }
    let content_range = format!("bytes {start}-{end}/{length}");
    respond(
        task,
        url,
        206,
        &mime,
        &[("Content-Range", content_range)],
        &body,
    );
}

fn mime_type(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .and_then(|e| UTType::typeWithFilenameExtension(&ns(e)))
        .and_then(|t| t.preferredMIMEType())
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_owned())
}

fn respond(
    task: &ProtocolObject<dyn WKURLSchemeTask>,
    url: &NSURL,
    status: isize,
    mime: &str,
    extra_headers: &[(&str, String)],
    body: &[u8],
) {
    let mut headers: Vec<(&str, String)> = vec![
        ("Content-Type", mime.to_owned()),
        ("Content-Length", body.len().to_string()),
        ("Accept-Ranges", "bytes".to_owned()),
        ("Cache-Control", "no-cache".to_owned()),
    ];
    headers.extend(extra_headers.iter().cloned());
    let keys: Vec<Retained<NSString>> = headers.iter().map(|(k, _)| ns(k)).collect();
    let values: Vec<Retained<NSString>> = headers.iter().map(|(_, v)| ns(v)).collect();
    let keys: Vec<&NSString> = keys.iter().map(|k| &**k).collect();
    let fields = NSDictionary::from_retained_objects(&keys, &values);
    let response = NSHTTPURLResponse::initWithURL_statusCode_HTTPVersion_headerFields(
        NSHTTPURLResponse::alloc(),
        url,
        status,
        Some(&ns("HTTP/1.1")),
        Some(&fields),
    );
    let Some(response) = response else {
        return fail(task);
    };
    unsafe {
        task.didReceiveResponse(&response);
        task.didReceiveData(&NSData::with_bytes(body));
        task.didFinish();
    }
}

fn fail(task: &ProtocolObject<dyn WKURLSchemeTask>) {
    let error = NSError::new(FILE_DOES_NOT_EXIST, unsafe { NSURLErrorDomain });
    unsafe { task.didFailWithError(&error) };
}
