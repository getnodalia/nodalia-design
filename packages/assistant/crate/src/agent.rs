//! The assistant's state and its tool-using loop.
//!
//! [`Assistant`] lives on the app's own thread. When the person sends a message it starts a worker
//! thread that talks to the provider; whenever the model asks for a tool the worker hands the call
//! back and waits, and the app thread runs it between frames through [`Host::call`]. So the
//! document is only ever touched by the app, and the network never blocks the window.
//!
//! A tool that changes the document waits for the person (Apply, Skip or Apply all) unless they
//! chose to let the assistant go ahead. At the end of a turn the undo steps it added are counted,
//! and one click takes them all back.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::host::Host;
use crate::lang::Lang;
use crate::message::{for_request, Message, ToolCall, ToolResult, ToolSpec};
use crate::prompt;
use crate::provider::Provider;
use crate::secrets::{KeyState, Keys};
use crate::settings::{Approval, Settings};
use crate::transport::{self, Endpoint, Failure};
use crate::wire::Stop;

/// A model that keeps calling tools without answering is stopped after this many rounds.
const MAX_STEPS: usize = 30;
/// What a request may weigh, in characters, before the oldest turns are dropped.
const HISTORY_BUDGET: usize = 320_000;
/// The longest tool answer given to the model.
const MAX_TOOL_OUTPUT: usize = 30_000;
/// The longest document snapshot.
const MAX_CONTEXT: usize = 60_000;

/// Wakes the app so it draws a frame (`ctx.request_repaint()`).
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// Where a tool call is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolState {
    /// It changes the document and waits for the person.
    Waiting,
    Done,
    Failed(String),
    Skipped,
}

/// One tool call in the transcript.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolItem {
    pub name: String,
    /// One line saying what it does.
    pub summary: String,
    /// The arguments, for whoever wants to see exactly what will happen.
    pub detail: String,
    pub changes: bool,
    pub state: ToolState,
}

/// Something the transcript says that is not the model's or the person's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Stopped,
    /// The answer was cut by the model's own length limit.
    Truncated,
    Refused,
    /// The model answered nothing.
    Empty,
    /// The turn changed the document by `steps` undo steps.
    Applied {
        steps: usize,
        undone: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    User(String),
    Assistant(String),
    Tool(ToolItem),
    Error(Failure),
    Note(Note),
}

/// The provider's models, as far as they have been asked for.
#[derive(Clone, Debug, PartialEq)]
pub enum ModelList {
    Idle,
    Loading,
    Ready(Vec<String>),
    Failed(Failure),
}

enum Event {
    /// A message the worker added to the conversation.
    Push(Message),
    Say(String),
    Call(ToolCall),
    Note(Note),
    Finished,
    Failed(Failure),
}

/// The app's answer to a call, back to the worker.
struct Answer {
    call_id: String,
    content: String,
    is_error: bool,
}

struct Run {
    events: Receiver<Event>,
    answers: Sender<Answer>,
    cancel: Arc<AtomicBool>,
}

struct Awaiting {
    call: ToolCall,
    item: usize,
    decision: Option<Decision>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Decision {
    Apply,
    ApplyAll,
    Skip,
}

pub struct Assistant {
    pub settings: Settings,
    pub keys: Keys,
    pub lang: Lang,
    pub items: Vec<Item>,
    /// What is typed in the chat box.
    pub draft: String,
    /// Whether the chat panel is showing.
    pub open: bool,
    messages: Vec<Message>,
    run: Option<Run>,
    tools: Vec<ToolSpec>,
    awaiting: Option<Awaiting>,
    /// The turn's changes need no more questions (the person chose Apply all).
    turn_auto: bool,
    undo_base: usize,
    /// A message waiting for the keychain to answer.
    deferred: Option<String>,
    models: HashMap<Provider, ModelList>,
    models_tx: Sender<(Provider, Result<Vec<String>, Failure>)>,
    models_rx: Receiver<(Provider, Result<Vec<String>, Failure>)>,
    wake: Waker,
}

impl Assistant {
    pub fn new(wake: Waker) -> Self {
        Self::with_settings(Settings::load(), wake)
    }

    pub fn with_settings(settings: Settings, wake: Waker) -> Self {
        let (models_tx, models_rx) = channel();
        let mut a = Assistant {
            keys: Keys::new(wake.clone()),
            settings,
            lang: Lang::English,
            items: Vec::new(),
            draft: String::new(),
            open: false,
            messages: Vec::new(),
            run: None,
            tools: Vec::new(),
            awaiting: None,
            turn_auto: false,
            undo_base: 0,
            deferred: None,
            models: HashMap::new(),
            models_tx,
            models_rx,
            wake,
        };
        a.keys.ensure(a.settings.provider);
        a
    }

    /// True while a turn is running.
    pub fn busy(&self) -> bool {
        self.run.is_some() || self.deferred.is_some()
    }

    /// True while a change waits for Apply or Skip.
    pub fn waiting_for_approval(&self) -> bool {
        self.awaiting.as_ref().is_some_and(|a| a.decision.is_none())
    }

    /// Whether a request could go out now, and if not, what is missing.
    pub fn ready(&self) -> Result<(), Failure> {
        let p = self.settings.provider;
        if p.needs_key() && self.keys.state(p) != KeyState::Loading && self.keys.get(p).is_none() {
            return Err(Failure::NoKey);
        }
        if self.settings.model().is_empty() {
            return Err(Failure::NoModel);
        }
        Ok(())
    }

    fn endpoint(&self) -> Endpoint {
        let p = self.settings.provider;
        Endpoint {
            provider: p,
            base_url: self.settings.address(),
            api_key: self.keys.get(p).map(str::to_string),
        }
    }

    pub fn set_provider(&mut self, provider: Provider) {
        if self.settings.provider != provider {
            self.settings.provider = provider;
            self.keys.ensure(provider);
            self.save();
        }
    }

    /// Writes the settings; a failure is not worth stopping for.
    pub fn save(&self) {
        let _ = self.settings.save();
    }

    /// The models asked for from the current provider.
    pub fn models(&self) -> &ModelList {
        self.models
            .get(&self.settings.provider)
            .unwrap_or(&ModelList::Idle)
    }

    /// Asks the current provider for its models, which also checks the connection.
    pub fn refresh_models(&mut self) {
        let provider = self.settings.provider;
        if matches!(self.models.get(&provider), Some(ModelList::Loading)) {
            return;
        }
        if provider.needs_key() && self.keys.get(provider).is_none() {
            self.models
                .insert(provider, ModelList::Failed(Failure::NoKey));
            return;
        }
        self.models.insert(provider, ModelList::Loading);
        let ep = self.endpoint();
        let (tx, wake) = (self.models_tx.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let _ = tx.send((provider, transport::list_models(&ep)));
            wake();
        });
    }

    /// Starts a turn with `text`. The host's snapshot and undo depth are taken now.
    pub fn send(&mut self, host: &mut dyn Host, text: &str) {
        let text = text.trim();
        if text.is_empty() || self.busy() {
            return;
        }
        self.keys.ensure(self.settings.provider);
        if self.keys.state(self.settings.provider) == KeyState::Loading {
            // The keychain has not answered yet: send as soon as it does.
            self.deferred = Some(text.to_string());
            return;
        }
        self.start(host, text);
    }

    fn start(&mut self, host: &mut dyn Host, text: &str) {
        if let Err(f) = self.ready() {
            self.items.push(Item::User(text.to_string()));
            self.items.push(Item::Error(f));
            return;
        }
        let snapshot = host
            .context()
            .ok()
            .map(|c| prompt::context(host.app_name(), &c, MAX_CONTEXT));
        self.tools = host.tools();
        self.undo_base = host.undo_depth();
        self.turn_auto = self.settings.approval == Approval::Auto;
        self.items.push(Item::User(text.to_string()));
        self.messages.push(Message::user(text, snapshot));
        let system = prompt::system(host.app_name(), &host.guide(), self.lang.code());
        let (events_tx, events) = channel();
        let (answers, answers_rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            endpoint: self.endpoint(),
            model: self.settings.model(),
            system,
            messages: self.messages.clone(),
            tools: self.tools.clone(),
            events: events_tx,
            answers: answers_rx,
            cancel: cancel.clone(),
            wake: self.wake.clone(),
        };
        std::thread::spawn(move || work(job));
        self.run = Some(Run {
            events,
            answers,
            cancel,
        });
    }

    /// Stops the turn: what is running finishes, nothing new starts.
    pub fn stop(&mut self) {
        self.deferred = None;
        if let Some(run) = &self.run {
            run.cancel.store(true, Ordering::Relaxed);
        }
        if let Some(a) = self.awaiting.as_mut() {
            a.decision = Some(Decision::Skip);
        }
        (self.wake)();
    }

    pub fn approve(&mut self) {
        self.decide(Decision::Apply);
    }

    pub fn approve_all(&mut self) {
        self.decide(Decision::ApplyAll);
    }

    pub fn skip(&mut self) {
        self.decide(Decision::Skip);
    }

    fn decide(&mut self, d: Decision) {
        if let Some(a) = self.awaiting.as_mut() {
            a.decision.get_or_insert(d);
        }
        (self.wake)();
    }

    /// Forgets the conversation. Not while a turn runs.
    pub fn new_chat(&mut self) {
        if self.run.is_some() {
            return;
        }
        self.items.clear();
        self.messages.clear();
        self.awaiting = None;
    }

    /// Takes back the changes of the last turn.
    pub fn undo_turn(&mut self, host: &mut dyn Host) {
        let Some((index, steps)) =
            self.items
                .iter()
                .enumerate()
                .rev()
                .find_map(|(i, it)| match it {
                    Item::Note(Note::Applied {
                        steps,
                        undone: false,
                    }) => Some((i, *steps)),
                    _ => None,
                })
        else {
            return;
        };
        match host.undo(steps) {
            Ok(()) => {
                if let Some(Item::Note(Note::Applied { undone, .. })) = self.items.get_mut(index) {
                    *undone = true;
                }
            }
            Err(e) => self.items.push(Item::Error(Failure::Provider(e))),
        }
    }

    /// Call every frame. Applies what the worker and the keychain have reported and runs the
    /// tools the model asked for. True when the transcript changed, so the frame should be
    /// drawn again.
    pub fn poll(&mut self, host: &mut dyn Host) -> bool {
        let mut changed = self.keys.poll();
        while let Ok((p, result)) = self.models_rx.try_recv() {
            changed = true;
            self.models.insert(
                p,
                match result {
                    Ok(list) => ModelList::Ready(list),
                    Err(f) => ModelList::Failed(f),
                },
            );
        }
        if self.deferred.is_some() && self.keys.state(self.settings.provider) != KeyState::Loading {
            if let Some(text) = self.deferred.take() {
                self.start(host, &text);
                changed = true;
            }
        }
        // The worker's events, one call at a time: it waits for each answer.
        while self.awaiting.is_none() {
            let Some(run) = &self.run else { break };
            let Ok(event) = run.events.try_recv() else {
                break;
            };
            changed = true;
            match event {
                Event::Push(m) => self.messages.push(m),
                Event::Say(t) => self.items.push(Item::Assistant(t)),
                Event::Note(n) => self.items.push(Item::Note(n)),
                Event::Call(call) => self.begin_call(host, call),
                Event::Finished => self.finish(host, None),
                Event::Failed(f) => self.finish(host, Some(f)),
            }
        }
        if self.awaiting.as_ref().is_some_and(|a| a.decision.is_some()) {
            changed = true;
            self.carry_out(host);
        }
        changed
    }

    fn begin_call(&mut self, host: &mut dyn Host, call: ToolCall) {
        let spec = self.tools.iter().find(|t| t.name == call.name).cloned();
        let Some(spec) = spec else {
            self.items.push(Item::Tool(ToolItem {
                name: call.name.clone(),
                summary: call.name.clone(),
                detail: String::new(),
                changes: false,
                state: ToolState::Failed("No such tool.".into()),
            }));
            self.answer(Answer {
                call_id: call.id,
                content: format!(
                    "There is no tool called {}. The tools are: {}.",
                    call.name,
                    self.tools
                        .iter()
                        .map(|t| t.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                is_error: true,
            });
            return;
        };
        let summary = host
            .describe(&call.name, &call.input)
            .unwrap_or_else(|| summarize(&call.name, &call.input));
        let detail = serde_json::to_string_pretty(&call.input).unwrap_or_default();
        let changes = !spec.read_only;
        let waits = changes && !self.turn_auto;
        self.items.push(Item::Tool(ToolItem {
            name: call.name.clone(),
            summary,
            detail: crate::wire::truncate(&detail, 4000),
            changes,
            state: if waits {
                ToolState::Waiting
            } else {
                ToolState::Done
            },
        }));
        let item = self.items.len() - 1;
        if waits {
            self.awaiting = Some(Awaiting {
                call,
                item,
                decision: None,
            });
        } else {
            self.execute(host, call, item);
        }
    }

    fn carry_out(&mut self, host: &mut dyn Host) {
        let Some(a) = self.awaiting.take() else {
            return;
        };
        match a.decision {
            Some(Decision::Skip) | None => {
                self.set_state(a.item, ToolState::Skipped);
                self.answer(Answer {
                    call_id: a.call.id,
                    content: "The person chose not to apply this change. Nothing was changed. \
Ask what they would like instead."
                        .into(),
                    is_error: true,
                });
            }
            Some(d) => {
                if d == Decision::ApplyAll {
                    self.turn_auto = true;
                }
                self.execute(host, a.call, a.item);
            }
        }
    }

    fn execute(&mut self, host: &mut dyn Host, call: ToolCall, item: usize) {
        let (content, is_error) = match host.call(&call.name, &call.input) {
            Ok(v) => {
                self.set_state(item, ToolState::Done);
                (render(&v), false)
            }
            Err(e) => {
                self.set_state(item, ToolState::Failed(crate::wire::truncate(&e, 300)));
                (e, true)
            }
        };
        self.answer(Answer {
            call_id: call.id,
            content,
            is_error,
        });
    }

    fn set_state(&mut self, item: usize, state: ToolState) {
        if let Some(Item::Tool(t)) = self.items.get_mut(item) {
            t.state = state;
        }
    }

    fn answer(&self, a: Answer) {
        if let Some(run) = &self.run {
            let _ = run.answers.send(a);
        }
    }

    fn finish(&mut self, host: &mut dyn Host, failure: Option<Failure>) {
        self.run = None;
        self.awaiting = None;
        match failure {
            Some(Failure::Cancelled) => self.items.push(Item::Note(Note::Stopped)),
            Some(f) => self.items.push(Item::Error(f)),
            None => {}
        }
        let steps = host.undo_depth().saturating_sub(self.undo_base);
        if steps > 0 {
            self.items.push(Item::Note(Note::Applied {
                steps,
                undone: false,
            }));
        }
    }
}

/// The tool's output as the model reads it.
fn render(v: &Value) -> String {
    let text = match v {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    };
    if text.chars().count() <= MAX_TOOL_OUTPUT {
        return if text.is_empty() {
            "Done.".into()
        } else {
            text
        };
    }
    let kept: String = text.chars().take(MAX_TOOL_OUTPUT).collect();
    let more = text.chars().count() - MAX_TOOL_OUTPUT;
    format!("{kept}\n…[{more} more characters were cut; ask for a smaller part]")
}

/// `name(a=1, b="x")`, short.
pub fn summarize(name: &str, input: &Value) -> String {
    let args = match input.as_object() {
        Some(o) if !o.is_empty() => o
            .iter()
            .map(|(k, v)| {
                let v = match v {
                    Value::String(s) => format!("\"{}\"", crate::wire::truncate(s, 40)),
                    other => crate::wire::truncate(&other.to_string(), 40),
                };
                format!("{k}={v}")
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    };
    crate::wire::truncate(&format!("{name}({args})"), 160)
}

struct Job {
    endpoint: Endpoint,
    model: String,
    system: String,
    messages: Vec<Message>,
    tools: Vec<ToolSpec>,
    events: Sender<Event>,
    answers: Receiver<Answer>,
    cancel: Arc<AtomicBool>,
    wake: Waker,
}

/// The worker: model turn, tool calls, model turn… until the model has said its piece.
fn work(job: Job) {
    let send = |e: Event| {
        let _ = job.events.send(e);
        (job.wake)();
    };
    let mut messages = job.messages.clone();
    for _ in 0..MAX_STEPS {
        if job.cancel.load(Ordering::Relaxed) {
            return send(Event::Failed(Failure::Cancelled));
        }
        let request = for_request(&messages, HISTORY_BUDGET);
        let reply = match transport::complete(
            &job.endpoint,
            &job.model,
            &job.system,
            &request,
            &job.tools,
            &job.cancel,
        ) {
            Ok(r) => r,
            Err(f) => return send(Event::Failed(f)),
        };
        let assistant = Message::assistant(reply.text.clone(), reply.calls.clone());
        if !assistant.parts.is_empty() {
            messages.push(assistant.clone());
            send(Event::Push(assistant));
        }
        if !reply.text.trim().is_empty() {
            send(Event::Say(reply.text.clone()));
        }
        if reply.calls.is_empty() {
            match reply.stop {
                Stop::Length => send(Event::Note(Note::Truncated)),
                Stop::Refused => send(Event::Note(Note::Refused)),
                _ if reply.text.trim().is_empty() => send(Event::Note(Note::Empty)),
                _ => {}
            }
            return send(Event::Finished);
        }
        let mut results = Vec::new();
        for call in &reply.calls {
            let result = if job.cancel.load(Ordering::Relaxed) {
                cancelled(call)
            } else {
                send(Event::Call(call.clone()));
                wait_for(&job, call)
            };
            results.push(result);
        }
        let message = Message::tool_results(results);
        messages.push(message.clone());
        send(Event::Push(message));
    }
    send(Event::Failed(Failure::TooManySteps))
}

fn cancelled(call: &ToolCall) -> ToolResult {
    ToolResult {
        call_id: call.id.clone(),
        content: "The person stopped the assistant before this ran.".into(),
        is_error: true,
    }
}

/// Waits for the app's answer to `call`. A stop gives an answer too: the conversation must hold
/// a result for every call.
fn wait_for(job: &Job, call: &ToolCall) -> ToolResult {
    loop {
        match job.answers.recv_timeout(Duration::from_millis(200)) {
            Ok(a) if a.call_id == call.id => {
                return ToolResult {
                    call_id: a.call_id,
                    content: a.content,
                    is_error: a.is_error,
                }
            }
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => {
                if job.cancel.load(Ordering::Relaxed) {
                    return cancelled(call);
                }
            }
            Err(RecvTimeoutError::Disconnected) => return cancelled(call),
        }
    }
}

#[cfg(test)]
mod tests;
