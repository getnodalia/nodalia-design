//! The person's API keys, in the OS keychain.
//!
//! A key is never written to disk by this crate, never logged and never put in an error. The
//! keychain can be slow (an unlock prompt) or missing (a bare Linux session), so every access
//! runs on its own thread and the interface asks [`Keys`] what it knows. Without a keychain a
//! key lasts as long as the process, and [`Keys::state`] says so.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::provider::Provider;

const SERVICE: &str = "Nodalia Assistant";

/// What the interface can say about a provider's key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    /// The keychain is being asked.
    Loading,
    /// There is none.
    Missing,
    /// It is in the keychain.
    Stored,
    /// It could not be put in the keychain, so it is kept until the app closes.
    SessionOnly,
}

enum Done {
    Loaded(Provider, Result<Option<String>, String>),
    Saved(Provider, Result<(), String>),
}

/// The keys the interface has asked about so far.
pub struct Keys {
    cache: HashMap<Provider, Option<String>>,
    loading: HashSet<Provider>,
    session_only: HashSet<Provider>,
    /// The keychain's own complaint, for a line in Preferences.
    pub problem: Option<String>,
    tx: Sender<Done>,
    rx: Receiver<Done>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl Keys {
    pub fn new(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (tx, rx) = channel();
        Keys {
            cache: HashMap::new(),
            loading: HashSet::new(),
            session_only: HashSet::new(),
            problem: None,
            tx,
            rx,
            wake,
        }
    }

    /// Starts reading `provider`'s key unless it is already known or being read.
    pub fn ensure(&mut self, provider: Provider) {
        if !provider.needs_key() && provider != Provider::Custom {
            self.cache.entry(provider).or_insert(None);
            return;
        }
        if self.cache.contains_key(&provider) || !self.loading.insert(provider) {
            return;
        }
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Done::Loaded(provider, read(provider)));
            wake();
        });
    }

    /// The key, once known.
    pub fn get(&self, provider: Provider) -> Option<&str> {
        self.cache
            .get(&provider)
            .and_then(|k| k.as_deref())
            .filter(|k| !k.is_empty())
    }

    pub fn state(&self, provider: Provider) -> KeyState {
        if self.session_only.contains(&provider) && self.get(provider).is_some() {
            KeyState::SessionOnly
        } else if self.get(provider).is_some() {
            KeyState::Stored
        } else if self.cache.contains_key(&provider) {
            KeyState::Missing
        } else {
            KeyState::Loading
        }
    }

    /// Keeps `key` for this session and writes it to the keychain in the background. An empty
    /// key removes it.
    pub fn set(&mut self, provider: Provider, key: &str) {
        let key = key.trim().to_string();
        self.loading.remove(&provider);
        if key.is_empty() {
            self.cache.insert(provider, None);
            self.session_only.remove(&provider);
            let wake = self.wake.clone();
            std::thread::spawn(move || {
                let _ = delete(provider);
                wake();
            });
            return;
        }
        self.cache.insert(provider, Some(key.clone()));
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Done::Saved(provider, write(provider, &key)));
            wake();
        });
    }

    /// Applies what the background threads found. True when something changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(done) = self.rx.try_recv() {
            changed = true;
            match done {
                Done::Loaded(p, result) => {
                    self.loading.remove(&p);
                    match result {
                        // A key typed while the read was running wins.
                        Ok(key) => {
                            self.cache.entry(p).or_insert(key);
                        }
                        Err(e) => {
                            self.cache.entry(p).or_insert(None);
                            self.problem = Some(e);
                        }
                    }
                }
                Done::Saved(p, result) => match result {
                    Ok(()) => {
                        self.session_only.remove(&p);
                        self.problem = None;
                    }
                    Err(e) => {
                        self.session_only.insert(p);
                        self.problem = Some(e);
                    }
                },
            }
        }
        changed
    }
}

fn entry(provider: Provider) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, provider.id()).map_err(describe)
}

fn read(provider: Provider) -> Result<Option<String>, String> {
    match entry(provider)?.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(describe(e)),
    }
}

fn write(provider: Provider, key: &str) -> Result<(), String> {
    entry(provider)?.set_password(key).map_err(describe)
}

fn delete(provider: Provider) -> Result<(), String> {
    match entry(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(describe(e)),
    }
}

/// The keychain's error without anything that could contain a key: its own variants carry
/// platform messages only.
fn describe(e: keyring::Error) -> String {
    crate::wire::truncate(&e.to_string(), 200)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> Keys {
        Keys::new(Arc::new(|| {}))
    }

    #[test]
    fn a_provider_without_a_key_is_never_asked_of_the_keychain() {
        let mut k = keys();
        k.ensure(Provider::Ollama);
        assert_eq!(k.state(Provider::Ollama), KeyState::Missing);
    }

    #[test]
    fn a_key_typed_now_is_known_now() {
        let mut k = keys();
        k.cache.insert(Provider::Claude, None);
        // `set` also writes to the keychain on a thread; only the immediate state is checked.
        k.cache.insert(Provider::Claude, Some("sk-1".into()));
        assert_eq!(k.get(Provider::Claude), Some("sk-1"));
        assert_eq!(k.state(Provider::Claude), KeyState::Stored);
    }

    #[test]
    fn a_failed_write_leaves_the_key_for_the_session_and_says_so() {
        let mut k = keys();
        k.cache.insert(Provider::OpenAi, Some("sk".into()));
        k.tx.send(Done::Saved(Provider::OpenAi, Err("no keychain".into())))
            .unwrap();
        assert!(k.poll());
        assert_eq!(k.state(Provider::OpenAi), KeyState::SessionOnly);
        assert_eq!(k.problem.as_deref(), Some("no keychain"));
        assert_eq!(k.get(Provider::OpenAi), Some("sk"));
    }

    #[test]
    fn a_late_read_does_not_overwrite_a_key_typed_meanwhile() {
        let mut k = keys();
        k.cache.insert(Provider::Gemini, Some("typed".into()));
        k.tx.send(Done::Loaded(Provider::Gemini, Ok(Some("old".into()))))
            .unwrap();
        k.poll();
        assert_eq!(k.get(Provider::Gemini), Some("typed"));
    }

    #[test]
    fn a_missing_keychain_means_no_key_and_a_message() {
        let mut k = keys();
        k.loading.insert(Provider::Mistral);
        k.tx.send(Done::Loaded(
            Provider::Mistral,
            Err("no secret service".into()),
        ))
        .unwrap();
        k.poll();
        assert_eq!(k.state(Provider::Mistral), KeyState::Missing);
        assert!(k.problem.is_some());
    }

    #[test]
    fn unknown_means_loading() {
        assert_eq!(keys().state(Provider::Claude), KeyState::Loading);
    }
}
