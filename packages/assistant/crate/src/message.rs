//! The conversation, in a form that belongs to no provider. `wire` turns it into each API's JSON.

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

/// A tool the model asked to run.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    /// The provider's id for the call; the result must carry it back.
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// What a tool answered.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolResult {
    pub call_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(String),
    /// The state of the document at the start of a turn. Only the newest one is sent: older ones
    /// describe a document that has changed since.
    Context(String),
    ToolUse(ToolCall),
    ToolResult(ToolResult),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
}

impl Message {
    pub fn user(text: impl Into<String>, context: Option<String>) -> Self {
        let mut parts = Vec::new();
        if let Some(c) = context {
            parts.push(Part::Context(c));
        }
        parts.push(Part::Text(text.into()));
        Message {
            role: Role::User,
            parts,
        }
    }

    pub fn assistant(text: String, calls: Vec<ToolCall>) -> Self {
        let mut parts = Vec::new();
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        parts.extend(calls.into_iter().map(Part::ToolUse));
        Message {
            role: Role::Assistant,
            parts,
        }
    }

    pub fn tool_results(results: Vec<ToolResult>) -> Self {
        Message {
            role: Role::User,
            parts: results.into_iter().map(Part::ToolResult).collect(),
        }
    }

    /// The characters this message adds to a request, for keeping a conversation within limits.
    pub fn weight(&self) -> usize {
        self.parts
            .iter()
            .map(|p| match p {
                Part::Text(t) | Part::Context(t) => t.len(),
                Part::ToolUse(c) => c.name.len() + c.input.to_string().len(),
                Part::ToolResult(r) => r.content.len(),
            })
            .sum()
    }

    fn starts_a_turn(&self) -> bool {
        self.role == Role::User
            && self
                .parts
                .iter()
                .any(|p| matches!(p, Part::Text(_) | Part::Context(_)))
    }
}

/// What the model is allowed to call.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// A JSON Schema object for the input.
    pub schema: Value,
    /// Whether the tool leaves the document as it is. Only these run without being asked about.
    pub read_only: bool,
}

/// The conversation as a request carries it: only the newest turn keeps its document snapshot,
/// and the oldest turns are dropped, whole, when it grows past `budget` characters. A tool call
/// is never separated from its result.
pub fn for_request(messages: &[Message], budget: usize) -> Vec<Message> {
    let newest_context = messages
        .iter()
        .rposition(|m| m.parts.iter().any(|p| matches!(p, Part::Context(_))));
    let mut out: Vec<Message> = messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let mut m = m.clone();
            if Some(i) != newest_context {
                m.parts.retain(|p| !matches!(p, Part::Context(_)));
            }
            m
        })
        .collect();
    let mut total: usize = out.iter().map(Message::weight).sum();
    while total > budget {
        // Drop from the start up to the next turn boundary, keeping at least the last turn.
        let next = out
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, m)| m.starts_a_turn())
            .map(|(i, _)| i);
        let Some(next) = next else { break };
        let dropped: usize = out.drain(..next).map(|m| m.weight()).sum();
        total = total.saturating_sub(dropped);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn turn(text: &str, ctx: &str) -> Message {
        Message::user(text, Some(ctx.into()))
    }

    #[test]
    fn only_the_newest_snapshot_is_sent() {
        let msgs = vec![
            turn("one", "old state"),
            Message::assistant("ok".into(), vec![]),
            turn("two", "new state"),
        ];
        let sent = for_request(&msgs, usize::MAX);
        let contexts: Vec<_> = sent
            .iter()
            .flat_map(|m| m.parts.iter())
            .filter_map(|p| match p {
                Part::Context(c) => Some(c.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(contexts, vec!["new state"]);
    }

    #[test]
    fn old_turns_go_whole_and_the_last_one_stays() {
        let call = ToolCall {
            id: "a".into(),
            name: "read".into(),
            input: json!({}),
        };
        let msgs = vec![
            Message::user("first", None),
            Message::assistant(String::new(), vec![call]),
            Message::tool_results(vec![ToolResult {
                call_id: "a".into(),
                content: "x".repeat(500),
                is_error: false,
            }]),
            Message::assistant("done".into(), vec![]),
            Message::user("second", None),
        ];
        let sent = for_request(&msgs, 100);
        assert_eq!(sent.len(), 1);
        assert!(matches!(sent[0].parts.first(), Some(Part::Text(t)) if t == "second"));
    }

    #[test]
    fn a_tool_result_is_never_the_first_message() {
        let call = ToolCall {
            id: "a".into(),
            name: "read".into(),
            input: json!({}),
        };
        let msgs = vec![
            Message::user("first", None),
            Message::assistant(String::new(), vec![call]),
            Message::tool_results(vec![ToolResult {
                call_id: "a".into(),
                content: "y".repeat(500),
                is_error: false,
            }]),
            Message::assistant("done".into(), vec![]),
            Message::user("second", None),
            Message::assistant("ok".into(), vec![]),
        ];
        for budget in [0, 10, 100, 600, 100_000] {
            let sent = for_request(&msgs, budget);
            assert!(
                sent.first().is_some_and(Message::starts_a_turn),
                "budget {budget}"
            );
        }
    }
}
