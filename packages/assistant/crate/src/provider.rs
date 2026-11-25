//! The AI providers the assistant can talk to, and what is known about each.
//!
//! Nodalia runs no backend: the person picks a provider, a model and their own API key. Two wire
//! protocols cover every provider here (see [`Protocol`]).

/// How a provider's HTTP API is spoken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    /// Anthropic's Messages API.
    Anthropic,
    /// Chat Completions, which OpenAI, Gemini, Mistral, OpenRouter, Ollama and most local servers
    /// speak.
    OpenAi,
}

/// A provider the person can choose in Preferences.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Provider {
    Claude,
    OpenAi,
    Gemini,
    Mistral,
    OpenRouter,
    /// A model running on this computer.
    Ollama,
    /// Any server that speaks Chat Completions: LM Studio, vLLM, a company gateway.
    Custom,
}

impl Provider {
    pub const ALL: [Provider; 7] = [
        Provider::Claude,
        Provider::OpenAi,
        Provider::Gemini,
        Provider::Mistral,
        Provider::OpenRouter,
        Provider::Ollama,
        Provider::Custom,
    ];

    /// The stable id stored in the settings file and used as the keychain account.
    pub fn id(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::OpenAi => "openai",
            Provider::Gemini => "gemini",
            Provider::Mistral => "mistral",
            Provider::OpenRouter => "openrouter",
            Provider::Ollama => "ollama",
            Provider::Custom => "custom",
        }
    }

    pub fn from_id(id: &str) -> Option<Provider> {
        Provider::ALL.into_iter().find(|p| p.id() == id)
    }

    /// The name shown in Preferences. Product names are not translated.
    pub fn name(self) -> &'static str {
        match self {
            Provider::Claude => "Claude",
            Provider::OpenAi => "OpenAI",
            Provider::Gemini => "Google Gemini",
            Provider::Mistral => "Mistral AI",
            Provider::OpenRouter => "OpenRouter",
            Provider::Ollama => "Ollama",
            Provider::Custom => "OpenAI-compatible",
        }
    }

    /// The company or project behind it, for the line under the name.
    pub fn maker(self) -> &'static str {
        match self {
            Provider::Claude => "Anthropic",
            Provider::OpenAi => "OpenAI",
            Provider::Gemini => "Google",
            Provider::Mistral => "Mistral AI",
            Provider::OpenRouter => "OpenRouter",
            Provider::Ollama => "Ollama",
            Provider::Custom => "",
        }
    }

    pub fn protocol(self) -> Protocol {
        match self {
            Provider::Claude => Protocol::Anthropic,
            _ => Protocol::OpenAi,
        }
    }

    /// The server address up to, not including, the API's own paths (`/messages`,
    /// `/chat/completions`, `/models`).
    pub fn default_base_url(self) -> &'static str {
        match self {
            Provider::Claude => "https://api.anthropic.com/v1",
            Provider::OpenAi => "https://api.openai.com/v1",
            // Gemini's OpenAI-compatible endpoint, which supports tool calls.
            Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai",
            Provider::Mistral => "https://api.mistral.ai/v1",
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::Ollama => "http://localhost:11434/v1",
            Provider::Custom => "http://localhost:1234/v1",
        }
    }

    /// Whether the provider will not answer without a key.
    pub fn needs_key(self) -> bool {
        !matches!(self, Provider::Ollama | Provider::Custom)
    }

    /// Whether the person can change the server address (a local server, or a gateway).
    pub fn custom_address(self) -> bool {
        matches!(self, Provider::Ollama | Provider::Custom)
    }

    /// Where the person creates an API key, when there is such a page.
    pub fn key_page(self) -> Option<&'static str> {
        match self {
            Provider::Claude => Some("https://console.anthropic.com/settings/keys"),
            Provider::OpenAi => Some("https://platform.openai.com/api-keys"),
            Provider::Gemini => Some("https://aistudio.google.com/apikey"),
            Provider::Mistral => Some("https://console.mistral.ai/api-keys"),
            Provider::OpenRouter => Some("https://openrouter.ai/keys"),
            Provider::Ollama | Provider::Custom => None,
        }
    }

    /// The model used until the person picks one; empty when the provider's own list has to be
    /// asked (its model ids change faster than this crate can follow).
    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Claude => "claude-sonnet-5-5",
            _ => "",
        }
    }

    /// Models worth suggesting before the provider's list has been fetched.
    pub fn suggested_models(self) -> &'static [&'static str] {
        match self {
            Provider::Claude => &[
                "claude-sonnet-5-5",
                "claude-opus-5-5",
                "claude-haiku-5-5",
                "claude-fable-5-2",
            ],
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_round_trip() {
        for p in Provider::ALL {
            assert_eq!(Provider::from_id(p.id()), Some(p));
        }
        let mut ids: Vec<_> = Provider::ALL.iter().map(|p| p.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Provider::ALL.len());
        assert_eq!(Provider::from_id("nope"), None);
    }

    #[test]
    fn only_claude_speaks_the_messages_api() {
        for p in Provider::ALL {
            let expected = if p == Provider::Claude {
                Protocol::Anthropic
            } else {
                Protocol::OpenAi
            };
            assert_eq!(p.protocol(), expected, "{}", p.id());
        }
    }

    #[test]
    fn local_servers_need_no_key_and_hosted_ones_do() {
        assert!(!Provider::Ollama.needs_key());
        assert!(!Provider::Custom.needs_key());
        assert!(Provider::Claude.needs_key());
        assert!(Provider::OpenAi.needs_key());
        assert!(Provider::Gemini.needs_key());
    }

    #[test]
    fn hosted_providers_use_https_and_no_trailing_slash() {
        for p in Provider::ALL {
            let url = p.default_base_url();
            assert!(!url.ends_with('/'), "{url}");
            if p.needs_key() {
                assert!(url.starts_with("https://"), "{url}");
            }
        }
    }

    #[test]
    fn the_default_model_is_one_of_the_suggestions() {
        for p in Provider::ALL {
            let d = p.default_model();
            if !d.is_empty() {
                assert!(p.suggested_models().contains(&d));
            }
        }
    }
}
