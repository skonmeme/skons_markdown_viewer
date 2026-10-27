//! AI search (STANDARD §6.6): the batches of blocks sent to a model, the
//! prompt, the answers and the wire format of an external LLM.

use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::settings::ModelOption;

/// Overlap of the pieces of a block that is longer than one batch.
pub const PIECE_OVERLAP_TOKENS: usize = 256;

/// Tokens kept free on the model on this Mac for the schema of the answer
/// and the answer itself.
const ON_DEVICE_ANSWER_TOKENS: usize = 384;

/// Smallest batch, so that tiny windows still make progress.
const MIN_BATCH_TOKENS: usize = 64;

/// Instructions given to every model.
pub const INSTRUCTIONS: &str = "You find the parts of a document that are related to a search \
query. You get the query and numbered blocks of the document. A block may start with the \
headings it belongs to, in parentheses. Choose the blocks whose content is related to the \
meaning of the query, even when they do not contain its words. Choose no block when none is \
related.";

/// Added to the instructions for an external LLM, which returns plain text.
pub const EXTERNAL_ANSWER_FORMAT: &str = "Answer with JSON only, in the form \
{\"blocks\": [1, 3]}, listing the numbers of the chosen blocks.";

/// A run of text inside one block of the page, as collected by the page
/// script.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Block {
    pub text: String,
    /// Titles of the headings the block belongs to, outermost first.
    pub headings: Vec<String>,
    /// Whether the block is itself a heading.
    pub heading: bool,
}

/// The text of a block (or a piece of it) as given to the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Index of the block in the list collected from the page.
    pub block: usize,
    /// The headings joined with " > ".
    pub headings: String,
    pub text: String,
}

/// Items sent to the model in one request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Batch {
    pub items: Vec<Item>,
}

/// Estimates token counts from characters (STANDARD §6.6): one token for
/// each Chinese, Japanese or Korean character and one token for every few
/// other characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenEstimate {
    other_chars_per_token: usize,
}

impl TokenEstimate {
    /// The estimate of the standard for an external LLM: four other
    /// characters per token.
    pub const EXTERNAL: Self = Self {
        other_chars_per_token: 4,
    };

    /// A safer estimate for the small window of the model on this Mac;
    /// Apple counts three to four Latin characters per token.
    pub const ON_DEVICE: Self = Self {
        other_chars_per_token: 3,
    };

    fn units(self, c: char) -> usize {
        if is_cjk(c) {
            self.other_chars_per_token
        } else {
            1
        }
    }

    pub fn tokens(self, text: &str) -> usize {
        let units: usize = text.chars().map(|c| self.units(c)).sum();
        units.div_ceil(self.other_chars_per_token)
    }
}

/// Whether `c` is a Chinese, Japanese or Korean character (including Hangul,
/// kana and the CJK punctuation and full-width forms).
pub fn is_cjk(c: char) -> bool {
    const RANGES: [(u32, u32); 17] = [
        (0x1100, 0x11FF),
        (0x2E80, 0x2FDF),
        (0x3000, 0x303F),
        (0x3040, 0x30FF),
        (0x3100, 0x312F),
        (0x3130, 0x318F),
        (0x3190, 0x31FF),
        (0x3200, 0x33FF),
        (0x3400, 0x4DBF),
        (0x4E00, 0x9FFF),
        (0xA960, 0xA97F),
        (0xAC00, 0xD7AF),
        (0xD7B0, 0xD7FF),
        (0xF900, 0xFAFF),
        (0xFE30, 0xFE4F),
        (0xFF00, 0xFFEF),
        (0x20000, 0x323AF),
    ];
    let c = u32::from(c);
    RANGES
        .iter()
        .any(|&(start, end)| (start..=end).contains(&c))
}

/// Tokens of blocks that fit into one batch of the model on this Mac: its
/// context window less room for the instructions and the answer.
pub fn on_device_batch_tokens(context_size: usize, query: &str) -> usize {
    let estimate = TokenEstimate::ON_DEVICE;
    let room = estimate.tokens(INSTRUCTIONS)
        + estimate.tokens(&prompt_header(query))
        + ON_DEVICE_ANSWER_TOKENS;
    context_size.saturating_sub(room).max(MIN_BATCH_TOKENS)
}

/// Tokens of blocks that fit into one batch of an external LLM: 80 % of the
/// maximum tokens set in the AI settings.
pub fn external_batch_tokens(max_tokens: u32) -> usize {
    (max_tokens as usize * 4 / 5).max(MIN_BATCH_TOKENS)
}

/// Splits the blocks into batches of at most `budget` tokens (STANDARD §6.6).
///
/// Batches are split between blocks, and a heading starts a new batch when
/// the current one is more than half full. A block longer than a batch is
/// cut into pieces that overlap by [`PIECE_OVERLAP_TOKENS`], each of them a
/// batch of its own. Blocks without text are left out.
pub fn batches(blocks: &[Block], budget: usize, estimate: TokenEstimate) -> Vec<Batch> {
    let mut batches = Vec::new();
    let mut current = Batch::default();
    let mut used = 0;
    for (index, block) in blocks.iter().enumerate() {
        let text = compact(&block.text);
        if text.is_empty() {
            continue;
        }
        let headings: Vec<String> = block
            .headings
            .iter()
            .map(|h| compact(h))
            .filter(|h| !h.is_empty())
            .collect();
        let item = Item {
            block: index,
            headings: headings.join(" > "),
            text,
        };
        let cost = item_tokens(&item, estimate);
        if block.heading && used * 2 > budget && !current.items.is_empty() {
            batches.push(std::mem::take(&mut current));
            used = 0;
        }
        if cost > budget {
            if !current.items.is_empty() {
                batches.push(std::mem::take(&mut current));
                used = 0;
            }
            let overhead = item_tokens(
                &Item {
                    text: String::new(),
                    ..item.clone()
                },
                estimate,
            );
            let piece_budget = budget.saturating_sub(overhead).max(1);
            for piece in pieces(&item.text, piece_budget, PIECE_OVERLAP_TOKENS, estimate) {
                batches.push(Batch {
                    items: vec![Item {
                        text: piece,
                        ..item.clone()
                    }],
                });
            }
            continue;
        }
        if used + cost > budget && !current.items.is_empty() {
            batches.push(std::mem::take(&mut current));
            used = 0;
        }
        used += cost;
        current.items.push(item);
    }
    if !current.items.is_empty() {
        batches.push(current);
    }
    batches
}

/// Cuts `text` into pieces of at most `budget` tokens; each piece after the
/// first starts `overlap` tokens before the end of the previous one.
fn pieces(text: &str, budget: usize, overlap: usize, estimate: TokenEstimate) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let limit = budget * estimate.other_chars_per_token;
    let overlap_units = overlap * estimate.other_chars_per_token;
    let mut pieces = Vec::new();
    let mut start = 0;
    loop {
        let mut end = start;
        let mut units = 0;
        while end < chars.len() && units + estimate.units(chars[end]) <= limit {
            units += estimate.units(chars[end]);
            end += 1;
        }
        end = end.max(start + 1);
        pieces.push(chars[start..end].iter().collect());
        if end >= chars.len() {
            return pieces;
        }
        let mut next = end;
        let mut back = 0;
        while next > start + 1 && back + estimate.units(chars[next - 1]) <= overlap_units {
            back += estimate.units(chars[next - 1]);
            next -= 1;
        }
        start = next;
    }
}

fn compact(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn item_line(number: usize, item: &Item) -> String {
    if item.headings.is_empty() {
        format!("[{number}] {}", item.text)
    } else {
        format!("[{number}] ({}) {}", item.headings, item.text)
    }
}

/// Tokens of an item in a prompt, assuming a two-digit number.
fn item_tokens(item: &Item, estimate: TokenEstimate) -> usize {
    estimate.tokens(&item_line(99, item)) + 1
}

fn prompt_header(query: &str) -> String {
    format!("Query: {query}\n\nBlocks:\n")
}

/// The prompt for one batch: the query and the numbered items.
pub fn prompt(query: &str, batch: &Batch) -> String {
    let mut text = prompt_header(query);
    for (index, item) in batch.items.iter().enumerate() {
        text.push_str(&item_line(index + 1, item));
        text.push('\n');
    }
    text
}

/// Numbers (starting at 1) of the items chosen in an answer, in increasing
/// order. Numbers that do not belong to an item are dropped.
pub fn chosen_items(answer: &str, count: usize) -> Vec<usize> {
    // Reasoning models may think aloud before the answer.
    let answer = answer.rsplit("</think>").next().unwrap_or(answer);
    let numbers = json_numbers(answer).unwrap_or_else(|| plain_numbers(answer));
    let mut chosen: Vec<usize> = numbers
        .into_iter()
        .filter(|n| (1..=count).contains(n))
        .collect();
    chosen.sort_unstable();
    chosen.dedup();
    chosen
}

fn json_numbers(answer: &str) -> Option<Vec<usize>> {
    if let (Some(start), Some(end)) = (answer.find('{'), answer.rfind('}'))
        && start < end
        && let Ok(Value::Object(object)) = serde_json::from_str::<Value>(&answer[start..=end])
        && let Some(Value::Array(values)) = object.get("blocks")
    {
        return Some(numbers_of(values));
    }
    if let (Some(start), Some(end)) = (answer.find('['), answer.rfind(']'))
        && start < end
        && let Ok(Value::Array(values)) = serde_json::from_str::<Value>(&answer[start..=end])
    {
        return Some(numbers_of(&values));
    }
    None
}

fn numbers_of(values: &[Value]) -> Vec<usize> {
    values
        .iter()
        .filter_map(|value| match value {
            Value::Number(n) => n.as_u64().map(|n| n as usize),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        })
        .collect()
}

fn plain_numbers(answer: &str) -> Vec<usize> {
    answer
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// The body of a Chat Completions request to an external LLM.
///
/// Each model option becomes a member of the body; a value that reads as
/// JSON is sent as that value and any other value as a string. Options with
/// an empty key are ignored, a later option wins over an earlier one with
/// the same key, and `model` and `messages` cannot be replaced.
pub fn request_body(model: &str, options: &[ModelOption], prompt: &str) -> String {
    let mut body = Map::new();
    body.insert("model".into(), Value::String(model.to_owned()));
    body.insert(
        "messages".into(),
        json!([
            {"role": "system", "content": format!("{INSTRUCTIONS} {EXTERNAL_ANSWER_FORMAT}")},
            {"role": "user", "content": prompt},
        ]),
    );
    for option in options {
        let key = option.key.trim();
        if key.is_empty() || key == "model" || key == "messages" {
            continue;
        }
        body.insert(key.to_owned(), option_value(&option.value));
    }
    Value::Object(body).to_string()
}

/// The JSON value of a model option.
pub fn option_value(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_owned()))
}

/// The text of the first choice of a Chat Completions response.
pub fn response_text(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    match value
        .get("choices")?
        .get(0)?
        .get("message")?
        .get("content")?
    {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => Some(
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect(),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(text: &str) -> Block {
        Block {
            text: text.to_owned(),
            ..Block::default()
        }
    }

    fn heading(text: &str) -> Block {
        Block {
            text: text.to_owned(),
            heading: true,
            ..Block::default()
        }
    }

    #[test]
    fn token_estimates() {
        let external = TokenEstimate::EXTERNAL;
        assert_eq!(external.tokens(""), 0);
        assert_eq!(external.tokens("abcd"), 1);
        assert_eq!(external.tokens("abcde"), 2);
        assert_eq!(external.tokens("한글"), 2);
        assert_eq!(external.tokens("日本語"), 3);
        assert_eq!(external.tokens("ひらがな"), 4);
        assert_eq!(external.tokens("한a"), 2);
        assert_eq!(TokenEstimate::ON_DEVICE.tokens("abcdef"), 2);
        assert_eq!(TokenEstimate::ON_DEVICE.tokens("한글"), 2);
    }

    #[test]
    fn batch_sizes() {
        assert_eq!(external_batch_tokens(32 * 1024), 26_214);
        assert_eq!(external_batch_tokens(128 * 1024), 104_857);
        let budget = on_device_batch_tokens(4096, "query");
        assert!(budget > 3000 && budget < 3700, "{budget}");
    }

    #[test]
    fn blocks_fill_batches_in_order() {
        let blocks: Vec<Block> = (0..10)
            .map(|i| block(&format!("block {i} {}", "x".repeat(30))))
            .collect();
        let batches = batches(&blocks, 40, TokenEstimate::EXTERNAL);
        let order: Vec<usize> = batches
            .iter()
            .flat_map(|b| b.items.iter().map(|i| i.block))
            .collect();
        assert_eq!(order, (0..10).collect::<Vec<_>>());
        assert!(batches.len() > 1);
        for batch in &batches {
            let tokens: usize = batch
                .items
                .iter()
                .map(|i| item_tokens(i, TokenEstimate::EXTERNAL))
                .sum();
            assert!(tokens <= 40, "{tokens}");
        }
    }

    #[test]
    fn empty_blocks_are_left_out_and_text_is_compacted() {
        let blocks = vec![block("  "), block(" a\n\tb "), block("")];
        let batches = batches(&blocks, 100, TokenEstimate::EXTERNAL);
        assert_eq!(batches.len(), 1);
        assert_eq!(
            batches[0].items,
            vec![Item {
                block: 1,
                headings: String::new(),
                text: "a b".into()
            }]
        );
    }

    #[test]
    fn a_heading_starts_a_new_batch_when_more_than_half_full() {
        // Each block costs 7 tokens; the budget is 20.
        let long = "x".repeat(16);
        let blocks = vec![block(&long), block(&long), heading(&long), block(&long)];
        let batches = batches(&blocks, 20, TokenEstimate::EXTERNAL);
        let split: Vec<Vec<usize>> = batches
            .iter()
            .map(|b| b.items.iter().map(|i| i.block).collect())
            .collect();
        assert_eq!(split, vec![vec![0, 1], vec![2, 3]]);

        // Less than half full: the heading stays in the batch.
        let blocks = vec![block(&long), heading(&long), block(&long)];
        let split: Vec<Vec<usize>> = super::batches(&blocks, 30, TokenEstimate::EXTERNAL)
            .iter()
            .map(|b| b.items.iter().map(|i| i.block).collect())
            .collect();
        assert_eq!(split, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn long_blocks_are_cut_into_overlapping_pieces() {
        let text: String = (0..4000)
            .map(|i| char::from(b'a' + (i % 26) as u8))
            .collect();
        let blocks = vec![block("before"), block(&text), block("after")];
        let batches = batches(&blocks, 400, TokenEstimate::EXTERNAL);
        let pieces: Vec<&Item> = batches
            .iter()
            .flat_map(|b| &b.items)
            .filter(|i| i.block == 1)
            .collect();
        assert!(pieces.len() >= 3, "{}", pieces.len());
        assert!(
            batches
                .iter()
                .all(|b| b.items.iter().all(|i| i.block != 1) || b.items.len() == 1)
        );
        for pair in pieces.windows(2) {
            let overlap: String = pair[0]
                .text
                .chars()
                .rev()
                .take(PIECE_OVERLAP_TOKENS * 4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            assert!(
                pair[1].text.starts_with(&overlap),
                "pieces must overlap by 256 tokens"
            );
        }
        let total: usize = pieces.iter().map(|p| p.text.chars().count()).sum();
        assert!(total > 4000);
        assert_eq!(batches.first().unwrap().items[0].block, 0);
        assert_eq!(batches.last().unwrap().items[0].block, 2);
    }

    #[test]
    fn prompt_numbers_items_with_their_headings() {
        let blocks = vec![
            Block {
                text: "Run the installer.".into(),
                headings: vec!["Setup".into(), " macOS ".into()],
                heading: false,
            },
            block("Other"),
        ];
        let batches = batches(&blocks, 1000, TokenEstimate::EXTERNAL);
        assert_eq!(
            prompt("install", &batches[0]),
            "Query: install\n\nBlocks:\n[1] (Setup > macOS) Run the installer.\n[2] Other\n"
        );
    }

    #[test]
    fn answers() {
        assert_eq!(chosen_items("{\"blocks\": [3, 1, 3]}", 3), vec![1, 3]);
        assert_eq!(chosen_items("Sure: {\"blocks\": [\"2\", 9]}", 3), vec![2]);
        assert_eq!(chosen_items("[2, 1]", 3), vec![1, 2]);
        assert_eq!(chosen_items("1,3", 3), vec![1, 3]);
        assert_eq!(chosen_items("{\"blocks\": []}", 3), Vec::<usize>::new());
        assert_eq!(
            chosen_items("<think>maybe 1 or {2}</think>{\"blocks\": [3]}", 3),
            vec![3]
        );
        assert_eq!(chosen_items("none", 3), Vec::<usize>::new());
    }

    #[test]
    fn request_bodies() {
        let options = vec![
            ModelOption {
                key: "temperature".into(),
                value: "0".into(),
            },
            ModelOption {
                key: "reasoning_effort".into(),
                value: "low".into(),
            },
            ModelOption {
                key: "stop".into(),
                value: "[\"x\"]".into(),
            },
            ModelOption {
                key: " ".into(),
                value: "ignored".into(),
            },
            ModelOption {
                key: "model".into(),
                value: "other".into(),
            },
            ModelOption {
                key: "temperature".into(),
                value: "0.5".into(),
            },
        ];
        let body: Value = serde_json::from_str(&request_body("m1", &options, "P")).unwrap();
        assert_eq!(body["model"], "m1");
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["reasoning_effort"], "low");
        assert_eq!(body["stop"], json!(["x"]));
        assert_eq!(body["messages"][1], json!({"role": "user", "content": "P"}));
        assert!(
            body["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("\"blocks\"")
        );
        assert_eq!(body.as_object().unwrap().len(), 5);
    }

    #[test]
    fn option_values() {
        assert_eq!(option_value("true"), json!(true));
        assert_eq!(option_value("null"), Value::Null);
        assert_eq!(option_value("\"low\""), json!("low"));
        assert_eq!(option_value("{\"a\": 1}"), json!({"a": 1}));
        assert_eq!(option_value("low"), json!("low"));
        assert_eq!(option_value(""), json!(""));
    }

    #[test]
    fn response_texts() {
        let body =
            br#"{"choices":[{"message":{"role":"assistant","content":"{\"blocks\":[1]}"}}]}"#;
        assert_eq!(response_text(body).as_deref(), Some("{\"blocks\":[1]}"));
        let parts = br#"{"choices":[{"message":{"content":[{"type":"text","text":"[1"},{"type":"text","text":"]"}]}}]}"#;
        assert_eq!(response_text(parts).as_deref(), Some("[1]"));
        assert_eq!(response_text(b"{}"), None);
        assert_eq!(response_text(b"not json"), None);
    }
}
