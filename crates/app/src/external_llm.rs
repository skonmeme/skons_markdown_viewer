//! An external LLM with the OpenAI-compatible Chat Completions API
//! (STANDARD §6.6).

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2_foundation::{
    NSData, NSError, NSHTTPURLResponse, NSMutableURLRequest, NSURL, NSURLResponse, NSURLSession,
    NSURLSessionConfiguration, NSURLSessionDataTask,
};

use crate::util::ns;

/// A request fails when no answer comes within 5 minutes.
const TIMEOUT_SECONDS: f64 = 300.0;

/// `NSURLErrorCancelled` and `NSURLErrorTimedOut`.
const URL_ERROR_CANCELLED: isize = -999;
const URL_ERROR_TIMED_OUT: isize = -1001;

/// Why a request did not produce an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    NoUrl,
    Unreachable,
    Authentication,
    RateLimited,
    TimedOut,
    /// Another HTTP status than success.
    Status(isize),
    /// The answer is not a Chat Completions response.
    BadResponse,
}

impl Failure {
    /// Whether the failure concerns the server rather than one batch, so
    /// that AI search cannot be used (STANDARD §6.6).
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Failure::NoUrl
                | Failure::Unreachable
                | Failure::Authentication
                | Failure::RateLimited
                | Failure::TimedOut
        )
    }

    pub fn reason(&self) -> String {
        match self {
            Failure::NoUrl => "The URL of the external LLM is empty.".into(),
            Failure::Unreachable => "The server can't be reached.".into(),
            Failure::Authentication => "Authentication failed.".into(),
            Failure::RateLimited => "The rate limit was reached.".into(),
            Failure::TimedOut => "The request timed out.".into(),
            Failure::Status(code) => format!("The server answered with HTTP status {code}."),
            Failure::BadResponse => "The server's answer isn't a Chat Completions response.".into(),
        }
    }
}

pub type Answer = Result<String, Failure>;

type Done = Box<dyn FnOnce(Answer)>;

thread_local! {
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    static PENDING: RefCell<HashMap<u64, Done>> = RefCell::new(HashMap::new());
    static SESSION: OnceCell<Retained<NSURLSession>> = const { OnceCell::new() };
}

/// A running request.
pub struct Request {
    id: u64,
    task: Retained<NSURLSessionDataTask>,
}

impl Request {
    /// Stops the request; its completion is not called.
    pub fn cancel(&self) {
        PENDING.with(|pending| pending.borrow_mut().remove(&self.id));
        self.task.cancel();
    }
}

/// Posts `body` to `url`. `done` is called on the main thread with the text
/// of the first choice of the answer.
pub fn send(
    url: &str,
    api_key: &str,
    body: &str,
    done: impl FnOnce(Answer) + 'static,
) -> Result<Request, Failure> {
    if url.is_empty() {
        return Err(Failure::NoUrl);
    }
    let address = NSURL::URLWithString(&ns(url)).ok_or(Failure::Unreachable)?;
    let scheme = address.scheme().map(|s| s.to_string().to_ascii_lowercase());
    if !matches!(scheme.as_deref(), Some("http" | "https")) {
        return Err(Failure::Unreachable);
    }
    let request = NSMutableURLRequest::requestWithURL(&address);
    request.setHTTPMethod(&ns("POST"));
    request.setValue_forHTTPHeaderField(Some(&ns("application/json")), &ns("Content-Type"));
    if !api_key.is_empty() {
        request.setValue_forHTTPHeaderField(
            Some(&ns(&format!("Bearer {api_key}"))),
            &ns("Authorization"),
        );
    }
    request.setHTTPBody(Some(&NSData::with_bytes(body.as_bytes())));
    request.setTimeoutInterval(TIMEOUT_SECONDS);

    let id = NEXT_ID.with(|next| next.replace(next.get() + 1));
    PENDING.with(|pending| pending.borrow_mut().insert(id, Box::new(done)));
    // Runs on a background queue of the session.
    let handler = RcBlock::new(
        move |data: *mut NSData, response: *mut NSURLResponse, error: *mut NSError| {
            let answer = unsafe { outcome(data.as_ref(), response.as_ref(), error.as_ref()) };
            DispatchQueue::main().exec_async(move || {
                let done = PENDING.with(|pending| pending.borrow_mut().remove(&id));
                if let (Some(done), Some(answer)) = (done, answer) {
                    done(answer);
                }
            });
        },
    );
    let task = unsafe { session().dataTaskWithRequest_completionHandler(&request, &handler) };
    task.resume();
    Ok(Request { id, task })
}

/// The answer of a finished task, or `None` when it was cancelled.
fn outcome(
    data: Option<&NSData>,
    response: Option<&NSURLResponse>,
    error: Option<&NSError>,
) -> Option<Answer> {
    if let Some(error) = error {
        return match error.code() {
            URL_ERROR_CANCELLED => None,
            URL_ERROR_TIMED_OUT => Some(Err(Failure::TimedOut)),
            _ => Some(Err(Failure::Unreachable)),
        };
    }
    let status = response
        .and_then(|r| r.downcast_ref::<NSHTTPURLResponse>())
        .map(|r| r.statusCode())
        .unwrap_or_default();
    let body = data.map(|d| d.to_vec()).unwrap_or_default();
    Some(match status {
        200..=299 => smv_core::ai::response_text(&body).ok_or(Failure::BadResponse),
        401 | 403 => Err(Failure::Authentication),
        429 => Err(Failure::RateLimited),
        408 | 504 => Err(Failure::TimedOut),
        code => Err(Failure::Status(code)),
    })
}

fn session() -> Retained<NSURLSession> {
    SESSION.with(|cell| {
        cell.get_or_init(|| {
            let configuration = NSURLSessionConfiguration::ephemeralSessionConfiguration();
            configuration.setTimeoutIntervalForRequest(TIMEOUT_SECONDS);
            configuration.setTimeoutIntervalForResource(TIMEOUT_SECONDS);
            NSURLSession::sessionWithConfiguration(&configuration)
        })
        .clone()
    })
}
