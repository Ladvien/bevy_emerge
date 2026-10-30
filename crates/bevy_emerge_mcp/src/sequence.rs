//! **Waiting for a capture sequence to end.**

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::brp::Brp;

/// **Poll `emerge/capture_sequence_status` for sequence `id` until it is no longer running.**
pub async fn wait_for_sequence(editor: &Brp, id: u64, deadline: Instant) -> Result<Value, String> {
    loop {
        let answer = editor
            .call("emerge/capture_sequence_status", json!({"id": id}))
            .await?;
        let done = answer.get("status").and_then(|s| s.as_str()) != Some("running");
        if done || Instant::now() > deadline {
            return Ok(answer);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
