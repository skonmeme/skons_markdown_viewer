//! Apple Foundation Model on this Mac, through the Swift bridge in
//! `swift/FoundationModels.swift` (STANDARD §6.6).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};

use dispatch2::DispatchQueue;

unsafe extern "C" {
    fn smv_afm_availability() -> i32;
    fn smv_afm_context_size() -> isize;
    fn smv_afm_choose(
        instructions: *const c_char,
        prompt: *const c_char,
        id: u64,
        callback: extern "C" fn(u64, i32, *const c_char),
    ) -> *mut c_void;
    fn smv_afm_cancel(request: *mut c_void);
    fn smv_afm_release(request: *mut c_void);
}

/// Why the model cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    AppleIntelligenceOff,
    NotSupported,
    NotReady,
    Other,
}

impl Unavailable {
    pub fn reason(self) -> &'static str {
        match self {
            Unavailable::AppleIntelligenceOff => "Apple Intelligence is turned off.",
            Unavailable::NotSupported => "This Mac doesn't support Apple Intelligence.",
            Unavailable::NotReady => "The model isn't ready yet.",
            Unavailable::Other => "The model on this Mac isn't available.",
        }
    }
}

pub fn availability() -> Result<(), Unavailable> {
    match unsafe { smv_afm_availability() } {
        0 => Ok(()),
        1 => Err(Unavailable::AppleIntelligenceOff),
        2 => Err(Unavailable::NotSupported),
        3 => Err(Unavailable::NotReady),
        _ => Err(Unavailable::Other),
    }
}

/// The context window of the model in tokens.
pub fn context_size() -> usize {
    unsafe { smv_afm_context_size() }.max(0) as usize
}

/// Why a request did not produce an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The language, the safety rules or the answer format stopped the model.
    Unhandled,
    /// The batch did not fit into the context window.
    TooLong,
    NotReady,
    RateLimited,
    Other(String),
}

pub type Answer = Result<String, Failure>;

type Done = Box<dyn FnOnce(Answer)>;

thread_local! {
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    static PENDING: RefCell<HashMap<u64, Done>> = RefCell::new(HashMap::new());
}

/// A running request. Dropping it does not cancel it.
pub struct Request {
    id: u64,
    handle: *mut c_void,
}

impl Request {
    /// Stops the request; its completion is not called.
    pub fn cancel(&self) {
        PENDING.with(|pending| pending.borrow_mut().remove(&self.id));
        unsafe { smv_afm_cancel(self.handle) };
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        unsafe { smv_afm_release(self.handle) };
    }
}

/// Asks the model for the items of `prompt` related to its query. `done` is
/// called on the main thread with the chosen numbers separated by commas.
pub fn choose(instructions: &str, prompt: &str, done: impl FnOnce(Answer) + 'static) -> Request {
    let id = NEXT_ID.with(|next| next.replace(next.get() + 1));
    PENDING.with(|pending| pending.borrow_mut().insert(id, Box::new(done)));
    let instructions = c_string(instructions);
    let prompt = c_string(prompt);
    let handle = unsafe { smv_afm_choose(instructions.as_ptr(), prompt.as_ptr(), id, answered) };
    Request { id, handle }
}

fn c_string(text: &str) -> CString {
    CString::new(text.replace('\0', " ")).expect("NUL bytes were replaced")
}

/// Called by Swift on a background thread.
extern "C" fn answered(id: u64, status: i32, text: *const c_char) {
    let text = (!text.is_null()).then(|| {
        unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .into_owned()
    });
    DispatchQueue::main().exec_async(move || {
        let answer = match status {
            0 => Ok(text.unwrap_or_default()),
            // Cancelled: the request was removed from PENDING.
            1 => return,
            2 => Err(Failure::Unhandled),
            3 => Err(Failure::TooLong),
            4 => Err(Failure::NotReady),
            5 => Err(Failure::RateLimited),
            _ => Err(Failure::Other(text.unwrap_or_default())),
        };
        if let Some(done) = PENDING.with(|pending| pending.borrow_mut().remove(&id)) {
            done(answer);
        }
    });
}
