//! **The BRP client** — the MCP server's way into the running editor.
//!
//! **JSON-RPC 2.0 over HTTP POST**, which is what `bevy_remote::http` speaks since Bevy 0.16.
//! Every request is a POST of
//! `{"jsonrpc":"2.0","id":N,"method":…, "params":…}` to the BRP root URL, and the answer is the
//! JSON-RPC response object.
//!
//! **The editor is a separate process and this server never binds a socket.** The editor owns the
//! world; the MCP server asks it questions and tells it what to do. That is the whole reason the
//! editor's verbs are registered as BRP commands rather than being callable in-process.

use serde_json::{Value, json};

/// **A BRP client.**
#[derive(Debug, Clone)]
pub struct Brp {
    url: String,
    /// **The request counter, in an `Arc`** because `AtomicU64` is not `Clone` and the server is
    /// cloned into every tool call.
    next_id: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Brp {
    /// **A client for the editor at `url`.**
    pub fn new(url: impl Into<String>) -> Brp {
        Brp {
            url: url.into(),
            next_id: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// **The editor's URL, from `BEVY_BRP_PORT`.**
    ///
    /// **The same variable the editor reads** (`harness.rs`: `BEVY_BRP_PORT`, default 15702), so
    /// one knob points both ends at the same socket.
    pub fn from_env() -> Brp {
        let port = std::env::var("BEVY_BRP_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(15702);
        Brp::new(format!("http://127.0.0.1:{port}/"))
    }

    /// **Call one BRP method.**
    ///
    /// **An error, never a panic.** The editor may be closed, may not be ready, or may refuse the
    /// call — and an agent's tool call that finds no editor running is a *refusal with a
    /// sentence*, not a crash of the server.
    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        // **A short timeout, because a tool call an agent is waiting on should not hang.** The
        // editor answers a registered command in a frame or two; five seconds is a long time to
        // wait for one.
        // **A five-second cap, because a tool call an agent is waiting on should not hang.** The
        // editor answers a registered command in a frame or two; five seconds is a long time to
        // wait for one.
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_global(Some(std::time::Duration::from_secs(5)))
                .build(),
        );
        let response = agent.post(&self.url).send_json(&body).map_err(|e| {
            format!(
                "the editor did not answer `{method}` at {}: {e}. Is it running, and is \
                     BEVY_BRP_PORT the same on both sides?",
                self.url
            )
        })?;
        let text = response
            .into_body()
            .read_to_string()
            .map_err(|e| format!("the editor's answer to `{method}` was not readable: {e}"))?;
        let parsed: Value = serde_json::from_str(&text)
            .map_err(|e| format!("the editor's answer to `{method}` was not JSON: {e}"))?;
        // **A JSON-RPC error is a refusal, and it is reported as one.**
        if let Some(error) = parsed.get("error") {
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("the editor refused the call");
            return Err(format!("{method}: {message}"));
        }
        parsed
            .get("result")
            .cloned()
            .ok_or_else(|| format!("{method}: the editor's answer had no result"))
    }
}
