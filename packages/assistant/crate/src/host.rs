//! What an app gives the assistant. The crate knows nothing about documents: the app says what it
//! is, shows the model the document, offers tools and runs them.
//!
//! Every method is called on the app's own thread, between frames, so a tool can use the open
//! document directly. Nothing here is called from the network thread.

use serde_json::Value;

use crate::lang::Lang;
pub use crate::message::ToolSpec;

pub trait Host {
    /// The product's name as the person reads it: `Letra`, `Celdas`, `Lámina`.
    fn app_name(&self) -> &str;

    /// What the model must know about this app that it cannot work out: how a document is made
    /// up, what the tools mean, the traps. Plain text, a few short paragraphs.
    fn guide(&self) -> String;

    /// A compact picture of the open document and the selection, taken when the person sends a
    /// message. Large documents give an outline and the part around the selection; the model
    /// reads the rest with the read tools. The crate cuts anything past 60 000 characters.
    fn context(&mut self) -> Result<Value, String>;

    /// The tools the model may call. The read ones run without asking; the rest wait for the
    /// person unless they chose to let the assistant apply changes as it goes.
    fn tools(&self) -> Vec<ToolSpec>;

    /// Runs one tool. An error is given to the model as the tool's result, so the message should
    /// say what was wrong and what is allowed.
    fn call(&mut self, tool: &str, input: &Value) -> Result<Value, String>;

    /// One line saying what the call will do, for the approval card and the transcript. `None`
    /// falls back to the tool's name and its arguments.
    fn describe(&self, _tool: &str, _input: &Value) -> Option<String> {
        None
    }

    /// A few things worth asking, shown in an empty chat. They are the app's own words, so the
    /// app translates them.
    fn suggestions(&self, _lang: Lang) -> Vec<String> {
        Vec::new()
    }

    /// How many steps the open document's undo history holds now.
    fn undo_depth(&mut self) -> usize;

    /// Undoes the last `steps` steps.
    fn undo(&mut self, steps: usize) -> Result<(), String>;
}
