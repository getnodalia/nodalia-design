//! The HTTP side: one blocking call per model turn, made from the assistant's worker thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::Value;

use crate::message::{Message, ToolSpec};
use crate::provider::{Protocol, Provider};
use crate::wire::{self, Reply};

const USER_AGENT: &str = "Nodalia-Assistant";

/// A model turn can take minutes on a local model; a hosted one answers far sooner.
const TURN_TIMEOUT: Duration = Duration::from_secs(300);
const LIST_TIMEOUT: Duration = Duration::from_secs(30);

/// Where to send a request, and with what key. The key is only ever put in a header.
#[derive(Clone)]
pub struct Endpoint {
    pub provider: Provider,
    pub base_url: String,
    pub api_key: Option<String>,
}

impl std::fmt::Debug for Endpoint {
    // Never prints the key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "…"))
            .finish()
    }
}

/// Why a request did not give an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The provider needs a key and there is none.
    NoKey,
    /// No model chosen.
    NoModel,
    /// The key goes only to `https://` servers, or to this computer.
    InsecureAddress,
    /// The server could not be reached.
    Network(String),
    /// 401 or 403.
    Rejected(String),
    /// 429.
    RateLimited(String),
    /// Any other answer that says no.
    Provider(String),
    /// The answer was not what the protocol says.
    Malformed(String),
    /// The person pressed Stop.
    Cancelled,
    /// The model kept calling tools without ever answering.
    TooManySteps,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.describe(crate::lang::Lang::English))
    }
}

/// Whether `url` is somewhere a key may be sent: any `https://` address, or this computer.
pub fn key_may_travel(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("https://") {
        return true;
    }
    let Some(rest) = lower.strip_prefix("http://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or("");
    let host = if let Some(v6) = host.strip_prefix('[') {
        v6.split(']').next().unwrap_or("")
    } else {
        host.split(':').next().unwrap_or("")
    };
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}

fn path(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Anthropic => "/messages",
        Protocol::OpenAi => "/chat/completions",
    }
}

fn agent(timeout: Duration) -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(timeout))
        // The status is read here: a refusal's body says why.
        .http_status_as_error(false)
        .build()
        .into()
}

fn check_endpoint(ep: &Endpoint) -> Result<(), Failure> {
    if ep.provider.needs_key() && ep.api_key.as_deref().is_none_or(str::is_empty) {
        return Err(Failure::NoKey);
    }
    if ep.api_key.as_deref().is_some_and(|k| !k.is_empty()) && !key_may_travel(&ep.base_url) {
        return Err(Failure::InsecureAddress);
    }
    Ok(())
}

fn headers(
    request: ureq::RequestBuilder<ureq::typestate::WithBody>,
    ep: &Endpoint,
) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
    let request = request
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/json");
    let key = ep.api_key.as_deref().filter(|k| !k.is_empty());
    match (ep.provider.protocol(), key) {
        (Protocol::Anthropic, key) => {
            let request = request.header("anthropic-version", "2023-06-01");
            match key {
                Some(k) => request.header("x-api-key", k),
                None => request,
            }
        }
        (Protocol::OpenAi, Some(k)) => request.header("Authorization", &format!("Bearer {k}")),
        (Protocol::OpenAi, None) => request,
    }
}

fn classify(status: u16, body: &Value) -> Failure {
    let message = wire::error_text(Some(status), body);
    match status {
        401 | 403 => Failure::Rejected(message),
        429 => Failure::RateLimited(message),
        _ => Failure::Provider(message),
    }
}

/// One model turn: the conversation so far, and what the model said.
pub fn complete(
    ep: &Endpoint,
    model: &str,
    system: &str,
    messages: &[Message],
    tools: &[ToolSpec],
    cancel: &AtomicBool,
) -> Result<Reply, Failure> {
    check_endpoint(ep)?;
    if model.trim().is_empty() {
        return Err(Failure::NoModel);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(Failure::Cancelled);
    }
    let protocol = ep.provider.protocol();
    let body = wire::request(protocol, model, system, messages, tools).to_string();
    let url = format!("{}{}", ep.base_url.trim_end_matches('/'), path(protocol));
    let request = headers(agent(TURN_TIMEOUT).post(&url), ep);
    let mut response = request
        .send(body.as_bytes())
        .map_err(|e| Failure::Network(network_text(&e.to_string())))?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| Failure::Network(network_text(&e.to_string())))?;
    if cancel.load(Ordering::Relaxed) {
        return Err(Failure::Cancelled);
    }
    let json: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status >= 400 {
        return Err(classify(status, &json));
    }
    wire::parse_reply(protocol, &json).map_err(Failure::Malformed)
}

/// The models the provider offers this key.
pub fn list_models(ep: &Endpoint) -> Result<Vec<String>, Failure> {
    check_endpoint(ep)?;
    let url = format!("{}/models", ep.base_url.trim_end_matches('/'));
    let mut request = agent(LIST_TIMEOUT)
        .get(&url)
        .header("User-Agent", USER_AGENT);
    let key = ep.api_key.as_deref().filter(|k| !k.is_empty());
    request = match (ep.provider.protocol(), key) {
        (Protocol::Anthropic, Some(k)) => request
            .header("anthropic-version", "2023-06-01")
            .header("x-api-key", k),
        (Protocol::OpenAi, Some(k)) => request.header("Authorization", &format!("Bearer {k}")),
        (_, None) => request,
    };
    let mut response = request
        .call()
        .map_err(|e| Failure::Network(network_text(&e.to_string())))?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| Failure::Network(network_text(&e.to_string())))?;
    let json: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status >= 400 {
        return Err(classify(status, &json));
    }
    Ok(wire::parse_models(&json))
}

/// An I/O error's text, cut short.
fn network_text(text: &str) -> String {
    wire::truncate(text, 200)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// Serves one canned answer on 127.0.0.1 and hands back the request it received.
    fn serve(status: &str, body: &str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        let status = status.to_string();
        let body = body.to_string();
        std::thread::spawn(move || {
            let Ok((mut s, _)) = listener.accept() else {
                return;
            };
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buf).to_string();
                if let Some(split) = text.find("\r\n\r\n") {
                    let head = text[..split].to_ascii_lowercase();
                    let len = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if buf.len() >= split + 4 + len {
                        break;
                    }
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(response.as_bytes());
        });
        (format!("http://{addr}"), rx)
    }

    fn endpoint(provider: Provider, base: String, key: Option<&str>) -> Endpoint {
        Endpoint {
            provider,
            base_url: base,
            api_key: key.map(str::to_string),
        }
    }

    #[test]
    fn keys_travel_only_over_https_or_to_this_computer() {
        for ok in [
            "https://api.anthropic.com/v1",
            "HTTPS://example.com",
            "http://localhost:11434/v1",
            "http://127.0.0.1:8080",
            "http://[::1]:8080/v1",
        ] {
            assert!(key_may_travel(ok), "{ok}");
        }
        for bad in [
            "http://example.com/v1",
            "http://192.168.1.4:1234/v1",
            "http://localhost.evil.com/v1",
            "http://127.0.0.1@evil.com/v1",
            "ftp://x",
            "",
        ] {
            assert!(!key_may_travel(bad), "{bad}");
        }
    }

    #[test]
    fn an_anthropic_turn_sends_the_key_in_its_header_and_reads_the_answer() {
        let (base, rx) = serve(
            "200 OK",
            r#"{"stop_reason":"end_turn","content":[{"type":"text","text":"Hi"}]}"#,
        );
        let ep = endpoint(Provider::Claude, base, Some("sk-test-123"));
        let reply = complete(
            &ep,
            "claude-x",
            "sys",
            &[Message::user("hello", None)],
            &[],
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(reply.text, "Hi");
        let req = rx.recv().unwrap().to_ascii_lowercase();
        assert!(req.starts_with("post /messages "), "{req}");
        assert!(req.contains("x-api-key: sk-test-123"));
        assert!(req.contains("anthropic-version: 2023-06-01"));
        assert!(req.contains("\"model\":\"claude-x\""));
    }

    #[test]
    fn an_openai_style_turn_uses_a_bearer_token() {
        let (base, rx) = serve(
            "200 OK",
            r#"{"choices":[{"finish_reason":"stop","message":{"content":"Hola"}}]}"#,
        );
        let ep = endpoint(Provider::Custom, base, Some("abc"));
        let reply = complete(
            &ep,
            "m",
            "sys",
            &[Message::user("hi", None)],
            &[],
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(reply.text, "Hola");
        let req = rx.recv().unwrap().to_ascii_lowercase();
        assert!(req.starts_with("post /chat/completions "), "{req}");
        assert!(req.contains("authorization: bearer abc"));
    }

    #[test]
    fn a_local_server_needs_no_key() {
        let (base, rx) = serve(
            "200 OK",
            r#"{"choices":[{"finish_reason":"stop","message":{"content":"ok"}}]}"#,
        );
        let ep = endpoint(Provider::Ollama, base, None);
        assert!(complete(
            &ep,
            "llama",
            "s",
            &[Message::user("x", None)],
            &[],
            &AtomicBool::new(false)
        )
        .is_ok());
        assert!(!rx
            .recv()
            .unwrap()
            .to_ascii_lowercase()
            .contains("authorization"));
    }

    #[test]
    fn refusals_are_classified_and_say_what_the_provider_said() {
        let (base, _rx) = serve(
            "401 Unauthorized",
            r#"{"error":{"message":"invalid x-api-key"}}"#,
        );
        let ep = endpoint(Provider::Custom, base, Some("k"));
        let err = complete(
            &ep,
            "m",
            "s",
            &[Message::user("x", None)],
            &[],
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(err, Failure::Rejected("invalid x-api-key".into()));
        let (base, _rx) = serve(
            "429 Too Many Requests",
            r#"{"error":{"message":"slow down"}}"#,
        );
        let ep = endpoint(Provider::Custom, base, None);
        let err = complete(
            &ep,
            "m",
            "s",
            &[Message::user("x", None)],
            &[],
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(err, Failure::RateLimited("slow down".into()));
    }

    #[test]
    fn nothing_is_sent_without_a_key_a_model_or_a_safe_address() {
        let never = AtomicBool::new(false);
        let m = [Message::user("x", None)];
        let no_key = endpoint(Provider::OpenAi, "https://api.openai.com/v1".into(), None);
        assert_eq!(
            complete(&no_key, "m", "s", &m, &[], &never).unwrap_err(),
            Failure::NoKey
        );
        let no_model = endpoint(Provider::Ollama, "http://localhost:1/v1".into(), None);
        assert_eq!(
            complete(&no_model, "  ", "s", &m, &[], &never).unwrap_err(),
            Failure::NoModel
        );
        let plain = endpoint(Provider::Custom, "http://example.com/v1".into(), Some("k"));
        assert_eq!(
            complete(&plain, "m", "s", &m, &[], &never).unwrap_err(),
            Failure::InsecureAddress
        );
        let stopped = AtomicBool::new(true);
        let ok = endpoint(Provider::Ollama, "http://localhost:1/v1".into(), None);
        assert_eq!(
            complete(&ok, "m", "s", &m, &[], &stopped).unwrap_err(),
            Failure::Cancelled
        );
    }

    #[test]
    fn the_models_list_is_read_with_the_same_key() {
        let (base, rx) = serve("200 OK", r#"{"data":[{"id":"b"},{"id":"a"}]}"#);
        let ep = endpoint(Provider::Custom, base, Some("k"));
        assert_eq!(list_models(&ep).unwrap(), vec!["a", "b"]);
        assert!(rx
            .recv()
            .unwrap()
            .to_ascii_lowercase()
            .starts_with("get /models "));
    }

    #[test]
    fn the_debug_form_hides_the_key() {
        let ep = endpoint(Provider::Claude, "https://x".into(), Some("sk-secret"));
        assert!(!format!("{ep:?}").contains("sk-secret"));
    }
}
