//! One AI assistant for the Nodalia apps.
//!
//! The person picks a provider (Claude, OpenAI, Gemini, Mistral, OpenRouter, a model on their own
//! computer through Ollama, or any compatible server), keeps their own API key in the system
//! keychain, and chats with a model that sees the open document and works on it through the
//! tools the app offers. Nodalia has no servers in between.
//!
//! The crate knows nothing about documents. An app implements [`Host`]: it names itself, gives
//! the model a snapshot of the document, lists its tools and runs them. [`Assistant`] does the
//! rest: the request, the loop, approval and undo. With the `egui` feature, [`ui`] draws the
//! button, the chat panel and the Preferences section the same way in every app.
//!
//! ```no_run
//! # fn demo(host: &mut dyn nodalia_assistant::Host) {
//! use std::sync::Arc;
//! use nodalia_assistant::Assistant;
//!
//! let mut assistant = Assistant::new(Arc::new(|| { /* ctx.request_repaint() */ }));
//! assistant.send(host, "Summarise this document");
//! // Every frame:
//! if assistant.poll(host) { /* draw again */ }
//! # }
//! ```

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

pub mod agent;
pub mod host;
pub mod lang;
pub mod message;
pub mod prompt;
pub mod provider;
pub mod secrets;
pub mod settings;
pub mod transport;
pub mod wire;

#[cfg(feature = "egui")]
pub mod ui;

pub use agent::{Assistant, Item, ModelList, Note, ToolItem, ToolState, Waker};
pub use host::{Host, ToolSpec};
pub use lang::Lang;
pub use provider::{Protocol, Provider};
pub use secrets::{KeyState, Keys};
pub use settings::{Approval, Settings};
pub use transport::Failure;
