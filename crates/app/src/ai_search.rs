//! One AI search: the batches of a document sent one after another to the
//! chosen model (STANDARD §6.6).

use std::collections::{BTreeSet, VecDeque};

use smv_core::ai::{self, Batch, Block, TokenEstimate};
use smv_core::settings::{AiModel, ExternalLlm, Settings};

use crate::{afm, external_llm};

/// The result of one batch.
pub enum BatchResult {
    /// Numbers (starting at 1) of the chosen items.
    Chosen(Vec<usize>),
    /// The batch did not fit into the context window of the model.
    TooLong,
    /// The model could not handle the batch.
    Failed(String),
    /// AI search cannot be used.
    Unavailable(String),
}

/// What a search does after a batch.
pub enum Progress {
    Searching,
    Finished,
    Unavailable(String),
}

enum Model {
    OnDevice,
    External(ExternalLlm),
}

enum Running {
    OnDevice(afm::Request),
    External(external_llm::Request),
}

impl Running {
    fn cancel(&self) {
        match self {
            Running::OnDevice(request) => request.cancel(),
            Running::External(request) => request.cancel(),
        }
    }
}

pub struct Search {
    model: Model,
    query: String,
    queue: VecDeque<Batch>,
    total: usize,
    done: usize,
    answered: usize,
    chosen: BTreeSet<usize>,
    last_failure: Option<String>,
    running: Option<(Batch, Running)>,
}

impl Search {
    /// Prepares a search over `blocks`, or tells why AI search cannot be
    /// used with the current settings.
    pub fn new(settings: &Settings, query: &str, blocks: &[Block]) -> Result<Self, String> {
        let (model, budget, estimate) = match settings.ai_model {
            AiModel::OnDevice => {
                afm::availability().map_err(|unavailable| unavailable.reason().to_owned())?;
                let budget = ai::on_device_batch_tokens(afm::context_size(), query);
                (Model::OnDevice, budget, TokenEstimate::ON_DEVICE)
            }
            AiModel::External => {
                let llm = settings.external_llm.clone();
                if llm.url.is_empty() {
                    return Err(external_llm::Failure::NoUrl.reason());
                }
                let budget = ai::external_batch_tokens(llm.max_tokens);
                (Model::External(llm), budget, TokenEstimate::EXTERNAL)
            }
        };
        let queue = VecDeque::from(ai::batches(blocks, budget, estimate));
        Ok(Self {
            model,
            query: query.to_owned(),
            total: queue.len(),
            queue,
            done: 0,
            answered: 0,
            chosen: BTreeSet::new(),
            last_failure: None,
            running: None,
        })
    }

    /// Blocks chosen so far, in document order.
    pub fn chosen(&self) -> Vec<usize> {
        self.chosen.iter().copied().collect()
    }

    /// Finished and all batches.
    pub fn progress(&self) -> (usize, usize) {
        (self.done, self.total)
    }

    /// Sends the next batch; `done` receives its result on the main thread.
    /// Returns `Ok(false)` when no batch is left and the reason when the
    /// request cannot be sent.
    pub fn send_next(&mut self, done: impl FnOnce(BatchResult) + 'static) -> Result<bool, String> {
        let Some(batch) = self.queue.pop_front() else {
            return Ok(false);
        };
        let prompt = ai::prompt(&self.query, &batch);
        let count = batch.items.len();
        let running = match &self.model {
            Model::OnDevice => {
                Running::OnDevice(afm::choose(ai::INSTRUCTIONS, &prompt, move |answer| {
                    done(on_device_result(answer, count))
                }))
            }
            Model::External(llm) => {
                let body = ai::request_body(&llm.model, &llm.options, &prompt);
                let request = external_llm::send(&llm.url, &llm.api_key, &body, move |answer| {
                    done(external_result(answer, count))
                })
                .map_err(|failure| failure.reason())?;
                Running::External(request)
            }
        };
        self.running = Some((batch, running));
        Ok(true)
    }

    /// Records the result of the batch that was sent last.
    pub fn record(&mut self, result: BatchResult) -> Progress {
        let Some((batch, _)) = self.running.take() else {
            return Progress::Searching;
        };
        match result {
            BatchResult::Chosen(numbers) => {
                for number in numbers {
                    if let Some(item) = batch.items.get(number - 1) {
                        self.chosen.insert(item.block);
                    }
                }
                self.answered += 1;
                self.done += 1;
            }
            // The estimate was too low: send the two halves instead.
            BatchResult::TooLong if batch.items.len() > 1 => {
                let mut first = batch.items;
                let second = first.split_off(first.len() / 2);
                self.queue.push_front(Batch { items: second });
                self.queue.push_front(Batch { items: first });
                self.total += 1;
            }
            BatchResult::TooLong => {
                self.last_failure =
                    Some("A part of the document is too long for the model.".into());
                self.done += 1;
            }
            BatchResult::Failed(reason) => {
                self.last_failure = Some(reason);
                self.done += 1;
            }
            BatchResult::Unavailable(reason) => return Progress::Unavailable(reason),
        }
        if !self.queue.is_empty() {
            return Progress::Searching;
        }
        if self.answered == 0 && self.total > 0 {
            return Progress::Unavailable(
                self.last_failure
                    .clone()
                    .unwrap_or_else(|| "The model couldn't handle this document.".into()),
            );
        }
        Progress::Finished
    }

    /// Stops the request that is running.
    pub fn cancel(&mut self) {
        if let Some((_, running)) = self.running.take() {
            running.cancel();
        }
        self.queue.clear();
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn on_device_result(answer: afm::Answer, count: usize) -> BatchResult {
    match answer {
        Ok(text) => BatchResult::Chosen(ai::chosen_items(&text, count)),
        Err(afm::Failure::TooLong) => BatchResult::TooLong,
        Err(afm::Failure::NotReady) => {
            BatchResult::Unavailable(afm::Unavailable::NotReady.reason().into())
        }
        Err(afm::Failure::RateLimited) => {
            BatchResult::Unavailable("The rate limit was reached.".into())
        }
        Err(afm::Failure::Unhandled) => {
            BatchResult::Failed("The model couldn't handle a part of the document.".into())
        }
        Err(afm::Failure::Other(text)) => BatchResult::Failed(text),
    }
}

fn external_result(answer: external_llm::Answer, count: usize) -> BatchResult {
    match answer {
        Ok(text) => BatchResult::Chosen(ai::chosen_items(&text, count)),
        Err(failure) if failure.is_fatal() => BatchResult::Unavailable(failure.reason()),
        Err(failure) => BatchResult::Failed(failure.reason()),
    }
}

/// The counter while a search runs (STANDARD §6.6).
pub fn searching_status(done: usize, total: usize) -> String {
    if total > 1 {
        format!("Searching… {done}/{total}")
    } else {
        "Searching…".to_owned()
    }
}
