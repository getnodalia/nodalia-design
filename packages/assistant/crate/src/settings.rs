//! What the person chose in Preferences, kept in one small JSON file shared by the office apps,
//! so the provider and model are picked once. The file holds no secret.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::provider::Provider;

/// Whether the model's changes to the document wait for a click.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Approval {
    /// Show what a tool is about to do and wait for Apply or Skip.
    #[default]
    Ask,
    /// Apply as the model goes; the turn can still be undone as a whole.
    Auto,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub provider: Provider,
    /// The model chosen for each provider.
    pub models: HashMap<Provider, String>,
    /// The server address chosen for the providers that allow one.
    pub addresses: HashMap<Provider, String>,
    pub approval: Approval,
    /// Whether the person turned the assistant on. Off hides the button.
    pub enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            provider: Provider::Claude,
            models: HashMap::new(),
            addresses: HashMap::new(),
            approval: Approval::Ask,
            enabled: true,
        }
    }
}

impl Settings {
    pub fn model(&self) -> String {
        self.models
            .get(&self.provider)
            .cloned()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| self.provider.default_model().to_string())
    }

    pub fn set_model(&mut self, model: &str) {
        self.models.insert(self.provider, model.trim().to_string());
    }

    pub fn address(&self) -> String {
        self.addresses
            .get(&self.provider)
            .cloned()
            .filter(|a| !a.trim().is_empty())
            .unwrap_or_else(|| self.provider.default_base_url().to_string())
    }

    pub fn set_address(&mut self, address: &str) {
        self.addresses.insert(
            self.provider,
            address.trim().trim_end_matches('/').to_string(),
        );
    }

    pub fn to_json(&self) -> Value {
        let mut models: Vec<_> = self.models.iter().collect();
        models.sort_by_key(|(p, _)| p.id());
        let mut addresses: Vec<_> = self.addresses.iter().collect();
        addresses.sort_by_key(|(p, _)| p.id());
        json!({
            "provider": self.provider.id(),
            "models": models.into_iter().map(|(p, m)| (p.id().to_string(), json!(m))).collect::<serde_json::Map<_, _>>(),
            "addresses": addresses.into_iter().map(|(p, a)| (p.id().to_string(), json!(a))).collect::<serde_json::Map<_, _>>(),
            "approval": match self.approval { Approval::Ask => "ask", Approval::Auto => "auto" },
            "enabled": self.enabled,
        })
    }

    /// Reads settings, ignoring anything that is not understood: a file from a newer version, or
    /// one edited by hand, never stops the app.
    pub fn from_json(v: &Value) -> Settings {
        let mut s = Settings::default();
        if let Some(p) = v
            .get("provider")
            .and_then(Value::as_str)
            .and_then(Provider::from_id)
        {
            s.provider = p;
        }
        let read = |key: &str| -> HashMap<Provider, String> {
            v.get(key)
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(id, val)| Some((Provider::from_id(id)?, val.as_str()?.to_string())))
                .collect()
        };
        s.models = read("models");
        s.addresses = read("addresses");
        if v.get("approval").and_then(Value::as_str) == Some("auto") {
            s.approval = Approval::Auto;
        }
        if let Some(e) = v.get("enabled").and_then(Value::as_bool) {
            s.enabled = e;
        }
        s
    }

    pub fn load_from(path: &Path) -> Settings {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|v| Settings::from_json(&v))
            .unwrap_or_default()
    }

    /// Writes the file whole or not at all.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&self.to_json())?)?;
        std::fs::rename(&tmp, path)
    }

    pub fn load() -> Settings {
        default_path().map_or_else(Settings::default, |p| Settings::load_from(&p))
    }

    pub fn save(&self) -> std::io::Result<()> {
        match default_path() {
            Some(p) => self.save_to(&p),
            None => Ok(()),
        }
    }
}

/// `…/nodalia/assistant.json` in the person's configuration folder.
pub fn default_path() -> Option<PathBuf> {
    let var = |k: &str| {
        std::env::var_os(k)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    let base = if cfg!(target_os = "windows") {
        var("APPDATA")?
    } else if cfg!(target_os = "macos") {
        var("HOME")?.join("Library").join("Application Support")
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config")))?
    };
    Some(base.join("nodalia").join("assistant.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nodalia-assistant-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        dir.join("assistant.json")
    }

    #[test]
    fn what_was_saved_is_read_back() {
        let path = temp("roundtrip");
        let mut s = Settings {
            provider: Provider::Ollama,
            approval: Approval::Auto,
            enabled: false,
            ..Settings::default()
        };
        s.set_model(" llama3 ");
        s.set_address("http://localhost:9999/v1/");
        s.save_to(&path).unwrap();
        let back = Settings::load_from(&path);
        assert_eq!(back, s);
        assert_eq!(back.model(), "llama3");
        assert_eq!(back.address(), "http://localhost:9999/v1");
        assert!(!std::fs::read_to_string(&path).unwrap().contains("key"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn anything_unreadable_gives_the_defaults() {
        let path = temp("broken");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(Settings::load_from(&path), Settings::default());
        std::fs::write(
            &path,
            r#"{"provider": 7, "models": {"x": "y"}, "approval": 3}"#,
        )
        .unwrap();
        assert_eq!(Settings::load_from(&path), Settings::default());
        assert_eq!(
            Settings::load_from(Path::new("/nonexistent/none.json")),
            Settings::default()
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn the_default_model_and_address_come_from_the_provider() {
        let s = Settings::default();
        assert_eq!(s.model(), "claude-sonnet-5-5");
        assert_eq!(s.address(), "https://api.anthropic.com/v1");
        let mut s = s;
        s.set_model("   ");
        assert_eq!(s.model(), "claude-sonnet-5-5");
    }

    #[test]
    fn each_provider_remembers_its_own_model() {
        let mut s = Settings::default();
        s.set_model("claude-opus-5-5");
        s.provider = Provider::OpenAi;
        assert_eq!(s.model(), "");
        s.set_model("gpt-x");
        s.provider = Provider::Claude;
        assert_eq!(s.model(), "claude-opus-5-5");
    }
}
