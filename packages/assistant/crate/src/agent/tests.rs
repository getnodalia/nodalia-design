use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::*;

/// Serves one canned JSON answer per connection, in order, and records each request body.
fn serve(answers: Vec<Value>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for answer in answers {
            let Ok((mut s, _)) = listener.accept() else {
                return;
            };
            let mut buf = Vec::new();
            let mut chunk = [0u8; 8192];
            loop {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buf).to_string();
                if let Some(split) = text.find("\r\n\r\n") {
                    let len = text[..split]
                        .to_ascii_lowercase()
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:").map(str::to_string))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if buf.len() >= split + 4 + len {
                        let body = String::from_utf8_lossy(&buf[split + 4..]).to_string();
                        log.lock().unwrap().push(body);
                        break;
                    }
                }
            }
            let body = answer.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(response.as_bytes());
        }
    });
    (format!("http://{addr}/v1"), seen)
}

fn say(text: &str) -> Value {
    json!({"choices": [{"finish_reason": "stop", "message": {"role": "assistant", "content": text}}]})
}

fn call(id: &str, name: &str, args: Value) -> Value {
    json!({"choices": [{"finish_reason": "tool_calls", "message": {
        "role": "assistant", "content": "On it.",
        "tool_calls": [{"id": id, "type": "function",
            "function": {"name": name, "arguments": args.to_string()}}]
    }}]})
}

#[derive(Default)]
struct Doc {
    text: String,
    depth: usize,
    calls: Vec<String>,
    undone: Vec<usize>,
}

impl Host for Doc {
    fn app_name(&self) -> &str {
        "Letra"
    }
    fn guide(&self) -> String {
        "A document is text.".into()
    }
    fn context(&mut self) -> Result<Value, String> {
        Ok(json!({"text": self.text}))
    }
    fn tools(&self) -> Vec<ToolSpec> {
        vec![
            ToolSpec {
                name: "read".into(),
                description: "Reads the document.".into(),
                schema: json!({"type": "object", "properties": {}}),
                read_only: true,
            },
            ToolSpec {
                name: "append".into(),
                description: "Adds text.".into(),
                schema: json!({"type": "object", "properties": {"text": {"type": "string"}}}),
                read_only: false,
            },
        ]
    }
    fn call(&mut self, tool: &str, input: &Value) -> Result<Value, String> {
        self.calls.push(tool.to_string());
        match tool {
            "read" => Ok(json!({"text": self.text})),
            "append" => {
                let t = input
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or("text is required")?;
                self.text.push_str(t);
                self.depth += 1;
                Ok(json!("added"))
            }
            _ => Err("unknown".into()),
        }
    }
    fn undo_depth(&mut self) -> usize {
        self.depth
    }
    fn undo(&mut self, steps: usize) -> Result<(), String> {
        self.undone.push(steps);
        self.depth -= steps;
        Ok(())
    }
}

fn assistant(address: &str, approval: Approval) -> Assistant {
    let mut settings = Settings {
        provider: Provider::Ollama,
        approval,
        ..Settings::default()
    };
    settings.set_model("test-model");
    settings.set_address(address);
    Assistant::with_settings(settings, Arc::new(|| {}))
}

/// Polls until `done` holds, failing after a few seconds.
fn pump(a: &mut Assistant, host: &mut Doc, done: impl Fn(&Assistant) -> bool) {
    let start = Instant::now();
    while !done(a) {
        a.poll(host);
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "timed out: {:?}",
            a.items
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn tools_in(a: &Assistant) -> Vec<&ToolItem> {
    a.items
        .iter()
        .filter_map(|i| match i {
            Item::Tool(t) => Some(t),
            _ => None,
        })
        .collect()
}

#[test]
fn a_plain_answer_is_shown_and_the_document_is_sent_as_context() {
    let (url, seen) = serve(vec![say("Hello there.")]);
    let mut a = assistant(&url, Approval::Ask);
    let mut doc = Doc {
        text: "Dear reader".into(),
        ..Doc::default()
    };
    a.send(&mut doc, "  Hi  ");
    assert!(a.busy());
    pump(&mut a, &mut doc, |a| !a.busy());
    assert_eq!(
        a.items,
        vec![
            Item::User("Hi".into()),
            Item::Assistant("Hello there.".into())
        ]
    );
    let sent = seen.lock().unwrap()[0].clone();
    assert!(sent.contains("Dear reader"), "{sent}");
    assert!(sent.contains("test-model"));
}

#[test]
fn a_change_waits_for_apply_and_the_turn_can_be_undone() {
    let (url, seen) = serve(vec![
        call("c1", "append", json!({"text": "!"})),
        say("Done."),
    ]);
    let mut a = assistant(&url, Approval::Ask);
    let mut doc = Doc::default();
    a.send(&mut doc, "Add an exclamation mark");
    pump(&mut a, &mut doc, |a| a.waiting_for_approval());
    assert!(
        doc.calls.is_empty(),
        "nothing runs before the person agrees"
    );
    assert_eq!(tools_in(&a)[0].state, ToolState::Waiting);

    a.approve();
    pump(&mut a, &mut doc, |a| !a.busy());
    assert_eq!(doc.text, "!");
    assert_eq!(tools_in(&a)[0].state, ToolState::Done);
    assert!(a.items.contains(&Item::Note(Note::Applied {
        steps: 1,
        undone: false
    })));
    // The tool's result went back to the model.
    let second = seen.lock().unwrap()[1].clone();
    assert!(
        second.contains("added") && second.contains("c1"),
        "{second}"
    );

    a.undo_turn(&mut doc);
    assert_eq!(doc.undone, vec![1]);
    assert!(a.items.contains(&Item::Note(Note::Applied {
        steps: 1,
        undone: true
    })));
    // A second click finds nothing left to undo.
    a.undo_turn(&mut doc);
    assert_eq!(doc.undone, vec![1]);
}

#[test]
fn skipping_a_change_leaves_the_document_alone_and_tells_the_model() {
    let (url, seen) = serve(vec![
        call("c1", "append", json!({"text": "x"})),
        say("Understood."),
    ]);
    let mut a = assistant(&url, Approval::Ask);
    let mut doc = Doc::default();
    a.send(&mut doc, "Add x");
    pump(&mut a, &mut doc, |a| a.waiting_for_approval());
    a.skip();
    pump(&mut a, &mut doc, |a| !a.busy());
    assert!(doc.calls.is_empty());
    assert_eq!(tools_in(&a)[0].state, ToolState::Skipped);
    assert!(!a
        .items
        .iter()
        .any(|i| matches!(i, Item::Note(Note::Applied { .. }))));
    assert!(seen.lock().unwrap()[1].contains("chose not to apply"));
}

#[test]
fn reading_never_waits_and_auto_mode_applies_as_it_goes() {
    let (url, _) = serve(vec![
        call("r1", "read", json!({})),
        call("c1", "append", json!({"text": "a"})),
        say("Done."),
    ]);
    let mut a = assistant(&url, Approval::Auto);
    let mut doc = Doc::default();
    a.send(&mut doc, "Go");
    pump(&mut a, &mut doc, |a| !a.busy());
    assert_eq!(doc.calls, vec!["read", "append"]);
    assert_eq!(doc.text, "a");
    assert!(tools_in(&a).iter().all(|t| t.state == ToolState::Done));
}

#[test]
fn apply_all_stops_the_questions_for_the_rest_of_the_turn() {
    let (url, _) = serve(vec![
        call("c1", "append", json!({"text": "a"})),
        call("c2", "append", json!({"text": "b"})),
        say("Done."),
    ]);
    let mut a = assistant(&url, Approval::Ask);
    let mut doc = Doc::default();
    a.send(&mut doc, "Go");
    pump(&mut a, &mut doc, |a| a.waiting_for_approval());
    a.approve_all();
    pump(&mut a, &mut doc, |a| !a.busy());
    assert_eq!(doc.text, "ab");
    assert!(a.items.contains(&Item::Note(Note::Applied {
        steps: 2,
        undone: false
    })));
}

#[test]
fn an_unknown_tool_and_a_failing_tool_are_reported_not_fatal() {
    let (url, seen) = serve(vec![
        call("c1", "nope", json!({})),
        call("c2", "append", json!({})),
        say("Sorry."),
    ]);
    let mut a = assistant(&url, Approval::Auto);
    let mut doc = Doc::default();
    a.send(&mut doc, "Go");
    pump(&mut a, &mut doc, |a| !a.busy());
    let tools = tools_in(&a);
    assert!(matches!(tools[0].state, ToolState::Failed(_)));
    assert!(matches!(&tools[1].state, ToolState::Failed(m) if m.contains("text is required")));
    assert!(seen.lock().unwrap()[1].contains("There is no tool called nope"));
}

#[test]
fn a_provider_error_ends_the_turn_with_a_failure() {
    let (url, _) = serve(vec![json!({"error": {"message": "overloaded"}})]);
    let mut a = assistant(&url, Approval::Ask);
    let mut doc = Doc::default();
    a.send(&mut doc, "Hi");
    pump(&mut a, &mut doc, |a| !a.busy());
    assert!(
        matches!(a.items.last(), Some(Item::Error(Failure::Malformed(m))) if m.contains("overloaded"))
    );
}

#[test]
fn nothing_goes_out_without_a_model_and_a_new_chat_forgets_everything() {
    let mut a = assistant("http://127.0.0.1:1/v1", Approval::Ask);
    a.settings.models.clear();
    let mut doc = Doc::default();
    a.send(&mut doc, "Hi");
    assert!(!a.busy());
    assert_eq!(a.items.last(), Some(&Item::Error(Failure::NoModel)));
    a.new_chat();
    assert!(a.items.is_empty());
}

#[test]
fn summaries_are_short_and_readable() {
    let s = summarize("append", &json!({"text": "hello", "n": 3}));
    assert_eq!(s, "append(n=3, text=\"hello\")");
    assert_eq!(summarize("read", &json!({})), "read()");
    assert!(
        summarize("x", &json!({"t": "y".repeat(500)}))
            .chars()
            .count()
            <= 161
    );
}

#[test]
fn a_long_tool_answer_is_cut_and_says_so() {
    let big = Value::String("x".repeat(MAX_TOOL_OUTPUT + 50));
    let out = render(&big);
    assert!(out.contains("50 more characters"));
    assert_eq!(render(&Value::Null), "null");
    assert_eq!(render(&json!("")), "Done.");
}
