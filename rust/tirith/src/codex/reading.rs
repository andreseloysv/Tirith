//! Reading an exec turn's JSONL events: the answer's text, whether the turn completed, and why it failed.
use serde_json::Value;

#[derive(Default)]
pub(super) struct Reading {
    pub text: String,
    pub complete: bool,
    pub failure: Option<String>,
}

impl Reading {
    /// Takes one event. Each agent names its event kind its own way; the shapes do not collide.
    pub fn take(&mut self, msg: &Value) {
        match msg["payload_type"]
            .as_str()
            .or_else(|| msg["type"].as_str())
            .unwrap_or("")
        {
            "item.completed" if msg["item"]["type"] == "agent_message" => self
                .text
                .push_str(&format!("{}\n", msg["item"]["text"].as_str().unwrap_or(""))),
            "assistant.message" => super::said(msg)
                .into_iter()
                .for_each(|t| self.text.push_str(&format!("{t}\n"))),
            "result" if msg["exitCode"].is_number() => self.complete = msg["exitCode"] == 0,
            "session.error" => self.failure = msg["data"]["message"].as_str().map(String::from),
            "run.output.delta" => self
                .text
                .push_str(msg["payload"]["text"].as_str().unwrap_or("")),
            t if t.starts_with("run.terminal.") => self.terminal(msg),
            "turn.completed" => self.complete = true,
            "turn.failed" => {
                self.failure = Some(
                    msg["error"]["message"]
                        .as_str()
                        .unwrap_or("Codex turn failed")
                        .to_string(),
                )
            }
            "error" => {
                self.failure = Some(msg["message"].as_str().unwrap_or("Codex error").to_string())
            }
            _ => {}
        }
    }

    /// Muse's `run.terminal.*`: the final text, and a reason when it did not complete.
    fn terminal(&mut self, msg: &Value) {
        self.complete = msg["payload"]["terminal"] == "completed";
        self.text = msg["payload"]["text"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .unwrap_or(std::mem::take(&mut self.text));
        self.failure = (!self.complete)
            .then(|| {
                msg["payload"]["reason"]
                    .as_str()
                    .or_else(|| msg["payload"]["text"].as_str())
                    .unwrap_or("the turn did not complete")
                    .to_string()
            })
            .or(self.failure.take());
    }

    /// The answer once the process ended with `status`, or why there is none.
    pub fn answer(
        self,
        status: std::process::ExitStatus,
        name: &str,
        who: &str,
    ) -> Result<String, String> {
        let Reading {
            text,
            complete,
            failure,
        } = self;
        (complete && status.success())
            .then_some(())
            .ok_or_else(|| {
                failure.unwrap_or_else(|| {
                    format!(
                        "{name} ended without completing the turn ({status}); see logs/{who}.log"
                    )
                })
            })?;
        (!text.trim().is_empty())
            .then_some(())
            .ok_or_else(|| format!("{name} returned an empty answer"))?;
        Ok(text.trim().to_string())
    }
}
