//! **The MCP server** — the bevy_emerge editor, over the Model Context Protocol.
//!
//! # What it is for
//!
//! The editor's verbs used to live in **key handlers**. An agent has no keyboard, so the only way
//! to reach the editor was to synthesise keystrokes — which is a simulation of a person, and the
//! brief's rule is explicit: *"Never drive the UI by clicking."*
//!
//! This server is the other door. **It exposes the editor's operations as MCP tools**, and each
//! tool dispatches through the editor's own registry — the same function a key handler calls — so
//! an agent and a person are doing **one** thing, not two.
//!
//! # The tools
//!
//! | tool | what it does |
//! |---|---|
//! | `emerge_list_operations` | every operation, its parameters' schema, and how it is undone |
//! | `emerge_preview_operation` | what an operation would change; nothing changes |
//! | `emerge_apply_operation` | run one, and record it in the operation log |
//! | `emerge_read_oplog` | the log, newest last |
//!
//! **Four tools, and the set is small on purpose.** An agent's context is spent on the tools it
//! is offered, and a catalogue of ninety-six verbs is a catalogue of ninety-six things to choose
//! badly from. **The four are a discovery surface**: the first names what exists, and the second
//! and third run it. Later iterations add one tool per operation as the registry grows, and the
//! discovery tool goes away when it is not needed.
//!
//! # The transport
//!
//! **Stdio, and only the MCP specification.** The server speaks JSON-RPC to its client over stdin
//! and stdout, and it reaches the editor over BRP — which is the editor's own transport, and the
//! editor is a separate process that owns the world.
//!
//! **Every path is confined to the project directory.** The server takes `--project` and refuses
//! anything outside it; the editor writes the files and the server never touches the disk.

use std::path::PathBuf;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::service::ServiceExt;
use rmcp::{ErrorData, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use bevy_emerge_mcp::{brp, docs};

use brp::Brp;

/// **The parameters of `emerge_preview_operation` and `emerge_apply_operation`.**
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationParams {
    /// **The operation's name**, as `emerge_list_operations` gives it. For example
    /// `map_set_settings`.
    pub operation: String,
    /// **The operation's own parameters**, as that operation's schema describes them. For example
    /// `{"bounds": [64.0, 8.0, 64.0]}`.
    #[serde(default)]
    pub parameters: Value,
}

/// **The parameters of `emerge_apply_operation`.**
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyParams {
    /// **The operation's name**, as `emerge_list_operations` gives it.
    pub operation: String,
    /// **The operation's own parameters**, as that operation's schema describes them.
    #[serde(default)]
    pub parameters: Value,
    /// **Whether to preview instead of applying.** `true` changes nothing: the map, the file and
    /// the log are all left alone, and the answer says what would have changed.
    #[serde(default)]
    pub dry_run: bool,
}

/// **The parameters of `emerge_read_oplog`.**
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogParams {
    /// **Read from this sequence number on.** `0` is the beginning of the log.
    #[serde(default)]
    pub since_seq: u64,
    /// **How many entries at most**, 1 to 200. The default is 50, which is a screenful.
    #[serde(default = "fifty")]
    pub limit: usize,
}

/// **Fifty entries, which is a screenful.**
fn fifty() -> usize {
    50
}

/// **The server.**
#[derive(Clone)]
pub struct Emerge {
    /// **The editor's BRP endpoint.**
    editor: Brp,
    /// **The project directory, and the only place the server may point at.**
    project: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Emerge {
    /// **A server for the editor at `editor`, rooted at `project`.**
    pub fn new(editor: Brp, project: PathBuf) -> Self {
        let tool_router = Self::tool_router();
        Emerge {
            editor,
            project,
            tool_router,
        }
    }

    /// **The project's root, for a tool that names a file.**
    pub fn project(&self) -> &std::path::Path {
        &self.project
    }

    /// **Is the editor on the project this server was told about?**
    ///
    /// **The one thing the two processes have to agree on**, and the server cannot know it by
    /// itself: the editor is the one that knows which project it has open. **So the server asks
    /// the editor for its project root and compares**, and a mismatch is a refusal with a sentence
    /// rather than a change to somebody else's map.
    pub async fn check_project(&self) -> Result<(), String> {
        let told = self
            .project
            .canonicalize()
            .unwrap_or_else(|_| self.project.clone());
        let answer = self.editor.call("emerge/project_root", json!({})).await?;
        let editor_root = answer
            .get("project_root")
            .and_then(|v| v.as_str())
            .ok_or("the editor did not say which project it has open.")?;
        let editor_root = std::path::PathBuf::from(editor_root);
        let editor_root = editor_root.canonicalize().unwrap_or(editor_root);
        if editor_root != told {
            return Err(format!(
                "the editor has `{}` open and this server was told `{}`. Point both at the same \
                 project, or the changes would land in the wrong one.",
                editor_root.display(),
                told.display()
            ));
        }
        Ok(())
    }

    /// **`emerge_list_operations` — every operation, its schema, and how it is undone.**
    ///
    /// **A read.** It asks the running editor, so the list is the editor's own and cannot be out
    /// of date.
    #[tool(
        name = "emerge_list_operations",
        description = "Every operation the running bevy_emerge editor knows, with each one's \
                       parameters' JSON schema and how it is undone. Call this before \
                       emerge_apply_operation so the parameters are the ones the operation \
                       actually takes. Returns the list; it does not change anything. The undo \
                       column names the strategy an operation declares, and no tool performs an \
                       undo yet.",
        annotations(
            title = "List the editor's operations",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_operations(&self) -> Result<CallToolResult, ErrorData> {
        match self.editor.call("emerge/operation_list", json!({})).await {
            Ok(v) => Ok(CallToolResult::structured(v)),
            Err(e) => Ok(refusal(&e)),
        }
    }

    /// **`emerge_preview_operation` — what an operation would change, and nothing changed.**
    ///
    /// **A read.** The editor's `plan` runs and the answer comes back; the map, the file and the
    /// log are untouched.
    #[tool(
        name = "emerge_preview_operation",
        description = "What an operation would change, and nothing changed: the map, the data file \
                       and the operation log are all left alone. Use this to check a proposal \
                       before applying it. Returns what would change, or the refusal.",
        annotations(
            title = "Preview an editor operation",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn preview_operation(
        &self,
        Parameters(p): Parameters<OperationParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self
            .editor
            .call(
                "emerge/operation_preview",
                json!({"operation": p.operation, "parameters": p.parameters}),
            )
            .await
        {
            Ok(v) => Ok(CallToolResult::structured(v)),
            Err(e) => Ok(refusal(&e)),
        }
    }

    /// **`emerge_apply_operation` — run one operation.**
    ///
    /// **Not a read.** It changes the map, writes the data file, and appends a line to the
    /// operation log. **`dry_run` previews instead**, and nothing is written.
    #[tool(
        name = "emerge_apply_operation",
        description = "Run one of the editor's operations. Changes the open map, writes the data \
                       file, and appends a line to the operation log. Set dry_run to true to see \
                       what would change without changing it. Returns the operation's id, which \
                       can be cited in a report to the developer, and the log's own record of it. \
                       The operation replaces the values it names rather than adding to them, \
                       so it is marked destructive; there is no undo tool yet, and a change is \
                       reverted by editing the data file back or by restoring it from version \
                       control.",
        annotations(
            title = "Apply an editor operation",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn apply_operation(
        &self,
        Parameters(p): Parameters<ApplyParams>,
    ) -> Result<CallToolResult, ErrorData> {
        // **The project is checked before anything is changed.** A change to the wrong project is
        // worse than no change, and the server cannot know which project the editor has open
        // except by asking.
        if let Err(why) = self.check_project().await {
            return Ok(refusal(&why));
        }
        match self
            .editor
            .call(
                "emerge/operation_apply",
                json!({
                    "operation": p.operation,
                    "parameters": p.parameters,
                    "dry_run": p.dry_run,
                }),
            )
            .await
        {
            Ok(v) => {
                // **A refusal is an error, and the tool says so.** The editor answers a refusal
                // with `outcome: "refused"` and a reason, and the agent must be able to tell that
                // apart from a success without reading prose.
                let refused = v.get("outcome").and_then(|o| o.as_str()) == Some("refused");
                if refused {
                    Ok(CallToolResult::structured_error(v))
                } else {
                    Ok(CallToolResult::structured(v))
                }
            }
            Err(e) => Ok(refusal(&e)),
        }
    }

    /// **`emerge_read_oplog` — the operation log.**
    ///
    /// **A read.** Every operation writes one line, from a person or an agent, and an undo is a
    /// new line rather than a deletion — so the log is the whole history.
    #[tool(
        name = "emerge_read_oplog",
        description = "The editor's operation log, newest last. Every operation appends one line, \
                       whether a person or an agent did it. Read it to see what has been done to \
                       this project. A refusal is not logged, so a line here is a line about a \
                       change that happened.",
        annotations(
            title = "Read the operation log",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn read_oplog(
        &self,
        Parameters(p): Parameters<LogParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self
            .editor
            .call(
                "emerge/oplog_read",
                json!({"since_seq": p.since_seq, "limit": p.limit}),
            )
            .await
        {
            Ok(v) => Ok(CallToolResult::structured(v)),
            Err(e) => Ok(refusal(&e)),
        }
    }
}

/// **A refusal, as a tool result an agent can read.**
///
/// **`isError: true`, and the reason in both places** — the text content and the structured
/// content. An agent reads one and a human reads the other, and both need the sentence.
fn refusal(why: &str) -> CallToolResult {
    CallToolResult::structured_error(json!({"refused": why}))
}

#[tool_handler]
impl rmcp::ServerHandler for Emerge {
    /// **A tool that does not exist is refused with the list of the ones that do.**
    ///
    /// **The generated handler answers "tool not found"**, which tells an agent that it guessed
    /// wrong and nothing else — and an agent that has to guess again is an agent that guesses
    /// again. The brief's rule is *"Invalid calls are rejected atomically, with a message saying
    /// what was wrong and what to change."* **What to change is a list of names.**
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<rmcp::model::CallToolResponse, rmcp::ErrorData> {
        let name = request.name.as_ref();
        if self.tool_router.get(name).is_none() {
            let all = self.tool_router.list_all();
            let names: Vec<&str> = all.iter().map(|t| t.name.as_ref()).collect();
            let result = CallToolResult::structured_error(json!({
                "refused": format!(
                    "`{name}` is not a tool this server has. The tools are: {}.",
                    names.join(", ")
                ),
            }));
            return Ok(result.into());
        }
        // **And the router, for a tool that does exist.** The `#[tool_handler]` macro generates
        // this call when the impl block does not carry its own — and this impl block does, so
        // the router is called here.
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "This server drives the bevy_emerge world editor. Call emerge_list_operations first to \
             learn the operations and their parameters, then emerge_preview_operation to check a \
             change, and emerge_apply_operation to make it. Every applied operation is recorded in \
             the editor's operation log.",
        )
    }
}

/// **Serve on stdio, until the client goes away.**
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut project: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => project = args.next().map(PathBuf::from),
            "--emit-docs" => {
                // **The document is written to stdout, and the shell redirects it.** The server
                // never writes a file in the project — it does not own the project directory —
                // so the one write it can do is to a file the caller names.
                print!("{}", docs::render());
                return Ok(());
            }
            "--help" | "-h" => {
                println!(
                    "bevy_emerge_mcp — the bevy_emerge editor over the Model Context Protocol.\n\n\
                     USAGE:\n    bevy_emerge_mcp [--project <dir>]\n\n\
                     --project <dir>   the project directory the editor is working on. The server \
                     never writes to it itself; the editor does. Defaults to the current \
                     directory.\n\n\
                     The editor's BRP endpoint comes from BEVY_BRP_PORT (default 15702), which is \
                     the same variable the editor reads."
                );
                return Ok(());
            }
            other => {
                eprintln!("bevy_emerge_mcp: `{other}` is not an argument this server takes.");
                eprintln!("Try `--help`.");
                std::process::exit(2);
            }
        }
    }
    let project = project.unwrap_or_else(|| PathBuf::from("."));
    let server = Emerge::new(Brp::from_env(), project);
    // **The project is checked when a tool is called, not when the server starts.**
    //
    // **A server that refuses to start is a server an agent cannot ask anything.** The editor may
    // be closed when the server starts and opened later — or the server may be started first on
    // purpose — and a server that exits because nothing is listening has told the agent
    // nothing except that the editor was not up. **So the check is per call**, and a call with
    // no editor answers with a sentence.
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
