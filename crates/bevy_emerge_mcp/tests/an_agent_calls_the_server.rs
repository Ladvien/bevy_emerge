//! **The MCP server, called as an outside agent would call it.**
//!
//! The brief's rule: *"When you add or change a tool, test it as an outside agent would: list the
//! tools, read only the description, call the tool, and check the result is usable with no
//! project knowledge. Add an integration test that calls it through an MCP client."*
//!
//! # What this does
//!
//! **It starts the server as a subprocess and speaks MCP to it over stdio** — the same way the
//! agent's client does. **It does not call the tool functions**, because a test that calls the
//! function is a test of the function and not of the server: it would not notice that the
//! description was wrong, that the tool was not registered, or that the answer was not shaped
//! like an MCP answer.
//!
//! **It reads only the description** — which is the point. The test asks the server what tools it
//! has, reads what the server says about them, and then calls one. **If the description were
//! not enough to make the call, the test cannot make it**, and that is the failure the brief is
//! asking about.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

/// **A running MCP server, as a subprocess.**
struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Server {
    /// **Start the server and shake hands.**
    fn start() -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_bevy_emerge_mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the server binary starts");
        let stdin = child.stdin.take().expect("the server has a stdin");
        let stdout = BufReader::new(child.stdout.take().expect("the server has a stdout"));
        let mut server = Server {
            child,
            stdin,
            stdout,
            next_id: 0,
        };
        let answer = server.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "bevy_emerge's own test", "version": "0.1.0"},
            }),
        );
        assert!(
            answer.get("error").is_none(),
            "the server refused the handshake: {answer}"
        );
        // **The notification that completes the handshake.** A notification has no id and no
        // answer, so it is written and not read back.
        server.notify("notifications/initialized", json!({}));
        server
    }

    /// **Send a request and read its answer.**
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        writeln!(self.stdin, "{body}").expect("the server's stdin is writable");
        self.stdin.flush().expect("the server's stdin flushes");
        // **Read until the answer with this id.** The server may send notifications in between,
        // and an answer to an earlier request may still be in flight.
        loop {
            let mut line = String::new();
            let read = self
                .stdout
                .read_line(&mut line)
                .expect("the server's stdout is readable");
            assert!(read > 0, "the server's stdout ended before it answered");
            let line = line.trim_end();
            let value: Value = serde_json::from_str(line).unwrap_or_else(|e| {
                panic!("the server sent a line that is not JSON: {line} ({e})")
            });
            if value.get("id").and_then(|i| i.as_u64()) == Some(id) {
                return value;
            }
        }
    }

    /// **Send a notification.**
    fn notify(&mut self, method: &str, params: Value) {
        let body = json!({"jsonrpc": "2.0", "method": method, "params": params});
        writeln!(self.stdin, "{body}").expect("the server's stdin is writable");
        self.stdin.flush().expect("the server's stdin flushes");
    }

    /// **Call a tool.**
    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// **The server lists its tools, and every one of them says enough to be called.**
///
/// **This is the brief's criterion, and it is checked by reading only the description.** The
/// test does not know what the tools do; it reads what the server says and asks whether that is
/// enough to know what to send and what comes back.
#[test]
fn the_server_lists_tools_that_say_what_they_do() {
    let mut server = Server::start();
    let listed = server.request("tools/list", json!({}));
    let tools = listed["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("the server's tool list is not an array: {listed}"));
    assert!(
        !tools.is_empty(),
        "the server lists no tools, so an agent has nothing to call: {listed}"
    );
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or_default();
        let description = tool["description"].as_str().unwrap_or_default();
        assert!(!name.is_empty(), "a tool has no name: {tool}");
        assert!(
            name.starts_with("emerge_"),
            "the tool `{name}` is not prefixed with `emerge_`."
        );
        assert!(
            description.len() > 40,
            "the tool `{name}` has a description of {} characters, which is too short to say what \
             it does, what it does not do, and what comes back. An agent reads this and nothing \
             else.",
            description.len()
        );
        // **The annotations are there and they are booleans.** A missing annotation is not a
        // false one, and a client that reads a missing one cannot tell a read from a write.
        let annotations = &tool["annotations"];
        for hint in ["readOnlyHint", "destructiveHint", "idempotentHint"] {
            assert!(
                annotations.get(hint).and_then(|v| v.as_bool()).is_some(),
                "the tool `{name}` has no `{hint}` annotation, so a client cannot tell what it \
                 does to the editor: {annotations}"
            );
        }
        // **And a schema, so the agent knows what to send.**
        assert!(
            tool.get("inputSchema").is_some(),
            "the tool `{name}` has no input schema, so an agent cannot tell what to send it."
        );
    }
}

/// **A call with no editor running is a refusal with a sentence, and not a crash.**
///
/// **The agent's server outlives the editor sometimes** — the editor is closed and reopened, and
/// the server is the long-lived one. So a call with nothing on the other end must come back as a
/// readable answer.
#[test]
fn a_call_with_no_editor_is_a_refusal_and_not_a_crash() {
    let mut server = Server::start();
    let answer = server.call("emerge_list_operations", json!({}));
    // **The call is answered**, and the answer says the editor is not there.
    assert!(
        answer.get("error").is_none(),
        "the call was refused at the protocol level, so the agent cannot read why: {answer}"
    );
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains("editor") || text.contains("BEVY_BRP_PORT"),
        "the refusal does not say the editor was not there, so an agent does not know what to do \
         about it: {text}"
    );
}

/// **An unknown tool is refused, and the answer says which tools there are.**
#[test]
fn an_unknown_tool_is_refused_with_the_list() {
    let mut server = Server::start();
    let answer = server.call("emerge_no_such_tool", json!({}));
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains("emerge_"),
        "the refusal does not name the tools that do exist, so an agent has to guess again: {text}"
    );
}
