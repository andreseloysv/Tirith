//! What a call used, read from its answer as it passes: Anthropic's `message_start` and `message_delta` or `usage`, OpenAI's `usage`, the Responses API's `response.usage`; as a server-sent stream or as one JSON body.
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Tally {
    /// Fresh input: what no cache gave or took.
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub seen: bool,
    /// The partial last line of a stream.
    carry: String,
    /// The body of an answer that is no stream.
    body: Vec<u8>,
}

fn n(v: &Value, key: &str) -> Option<i64> {
    v[key].as_i64()
}

impl Tally {
    /// One JSON object of the answer.
    pub fn json(&mut self, v: &Value) {
        let kind = v["type"].as_str().unwrap_or("");
        let u = match kind {
            "message_start" => &v["message"]["usage"],
            "response.completed" | "response.done" => &v["response"]["usage"],
            _ if v["usage"].is_object() => &v["usage"],
            _ => return,
        };
        self.seen = true;
        if let Some(prompt) = n(u, "prompt_tokens") {
            let cached = u["prompt_tokens_details"]["cached_tokens"]
                .as_i64()
                .unwrap_or(0);
            (self.input, self.cache_read) = ((prompt - cached).max(0), cached);
            self.output = n(u, "completion_tokens").unwrap_or(0);
        } else if u["input_tokens_details"].is_object() {
            let cached = u["input_tokens_details"]["cached_tokens"]
                .as_i64()
                .unwrap_or(0);
            let input = n(u, "input_tokens").unwrap_or(0);
            (self.input, self.cache_read) = ((input - cached).max(0), cached);
            self.output = n(u, "output_tokens").unwrap_or(0);
        } else {
            let set = |dest: &mut i64, key: &str| {
                if let Some(x) = n(u, key).filter(|x| *x > 0 || kind != "message_delta") {
                    *dest = x;
                }
            };
            set(&mut self.input, "input_tokens");
            set(&mut self.output, "output_tokens");
            set(&mut self.cache_read, "cache_read_input_tokens");
            set(&mut self.cache_write, "cache_creation_input_tokens");
        }
    }

    /// The next bytes of the answer: in a stream, whole `data:` lines are read as they complete; any other body is kept (up to 16 MB) to be read at `end`.
    pub fn feed(&mut self, bytes: &[u8], stream: bool) {
        if !stream {
            let room = (16usize << 20).saturating_sub(self.body.len());
            self.body.extend_from_slice(&bytes[..bytes.len().min(room)]);
            return;
        }
        self.carry.push_str(&String::from_utf8_lossy(bytes));
        while let Some(at) = self.carry.find('\n') {
            let line: String = self.carry.drain(..=at).collect();
            self.data(&line);
        }
    }

    fn data(&mut self, line: &str) {
        let data = line.trim().strip_prefix("data:").map(str::trim);
        if let Some(v) = data.and_then(|d| serde_json::from_str::<Value>(d).ok()) {
            self.json(&v);
        }
    }

    /// The answer ended.
    pub fn end(&mut self) {
        let last = std::mem::take(&mut self.carry);
        self.data(&last);
        if let Ok(v) = serde_json::from_slice::<Value>(&std::mem::take(&mut self.body)) {
            self.json(&v);
        }
    }
}
