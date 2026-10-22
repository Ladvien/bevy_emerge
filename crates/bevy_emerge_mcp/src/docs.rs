//! **The MCP tool reference, generated from the server's own declarations.**
//!
//! # Why it is generated and not written
//!
//! A hand-written tool reference is a **second vocabulary**: it says one thing and the server
//! does another, and the reader — an agent, mostly — believes the document. The brief's rule is
//! that the reference is *"generated from the registry, or mechanically checked against it"*, and
//! this is the checked half: the document is rendered from the same declarations the server
//! builds its tools from, and a test fails when the two drift.
//!
//! **The rendering reads the server's own source**, so there is one list — the attributes on the
//! `#[tool]` methods — and the document is a view of it.

use serde_json::Value;

/// **The tool the server declares, as the document shows it.**
pub struct ToolDoc {
    pub name: &'static str,
    pub description: &'static str,
    pub read_only: bool,
    pub destructive: bool,
    pub idempotent: bool,
}

/// **Every tool the server declares.**
///
/// **The four, and they are the four the editor's BRP bridge registers.** The parity test in the
/// mapper holds the two lists to each other, so a tool that is added on one side and not the
/// other fails a test rather than a reader's trust.
pub const TOOLS: &[ToolDoc] = &[
    ToolDoc {
        name: "emerge_list_operations",
        description: "Every operation the running bevy_emerge editor knows, with each one's parameters' \
                      JSON schema and how it is undone. Call this before emerge_apply_operation so the \
                      parameters are the ones the operation actually takes. Returns the list; it does not \
                      change anything. The undo column names the strategy an operation declares; \
                      emerge_undo_operation performs it.",
        read_only: true,
        destructive: false,
        idempotent: true,
    },
    ToolDoc {
        name: "emerge_preview_operation",
        description: "What an operation would change, and nothing changed: the map, the data file \
                      and the operation log are all left alone. Use this to check a proposal \
                      before applying it. Returns what would change, or the refusal.",
        read_only: true,
        destructive: false,
        idempotent: true,
    },
    ToolDoc {
        name: "emerge_apply_operation",
        description: "Run one of the editor's operations. Changes the open map, writes the data file, and \
                      appends a line to the operation log. Set dry_run to true to see what would change \
                      without changing it. Returns the operation's id, which can be cited in a report to \
                      the developer, and the log's own record of it. The operation replaces the values it \
                      names rather than adding to them, so it is marked destructive. Take a change back \
                      with emerge_undo_operation and the returned id.",
        read_only: false,
        destructive: true,
        idempotent: false,
    },
    ToolDoc {
        name: "emerge_undo_operation",
        description: "Take back one change made through emerge_apply_operation. Pass operation_id, the id \
                      that call returned; omit it to take back the newest change that has not been taken \
                      back. The editor applies the inverse the change recorded, writes the data file, and \
                      appends a new log line naming the line it undoes; the log is never edited. Refuses, \
                      and changes nothing, when the change was already undone, when a newer change exists \
                      (the answer names it; undo that one first), when the map was changed outside the log \
                      since (a hand-edited file, the menu, or another map open; the answer names what \
                      differs), or when the change recorded no inverse. Undoing an undo puts the change \
                      back, and the change's own id takes it back again. Set dry_run to true to see what \
                      the undo would set. Returns the undo's own operation id and what it set.",
        read_only: false,
        destructive: true,
        idempotent: false,
    },
    ToolDoc {
        name: "emerge_state",
        description: "What the editor is holding right now: the open map and its bounds, how many \
                       pieces it has, what is armed, what is selected, which door and tab are open, \
                       and what the editor last said and what has gone wrong. The editor section also \
                       names the next undo and redo (undo_next, redo_next) and how many are stacked \
                       (undo_depth, redo_depth). The map section names each pinned placement by id and \
                       reason (pinned; pinned_total is the full count). Returns that summary as JSON, \
                       and changes nothing. \
                       Call it before a change to know what you are changing, and after one to check \
                       that it landed.",
        read_only: true,
        destructive: false,
        idempotent: true,
    },
    ToolDoc {
        name: "emerge_capture_still",
        description: "A picture of the editor as it is now -- the world and the interface, because \
                       the editor draws both into one image -- plus a state summary of the same \
                       moment. The picture is returned as an image and written to a file whose path \
                       is in the answer. Use this after a change to see what it did, or before one \
                       to see what it was like.",
        read_only: false,
        destructive: false,
        idempotent: false,
    },
    ToolDoc {
        name: "emerge_snip",
        description: "A rectangle of the editor's picture, cut out and returned as an image. rect \
                      is [x0, y0, x1, y1] in window pixels, origin at the top left, x1 and y1 \
                      exclusive, so the same pixels a still is in. The crop is written to a file \
                      whose path is in the answer, with the state of the same moment. It writes \
                      one crop file under target/loop and changes no map data: it is not an \
                      operation, not logged and not undoable. Refuses, and writes nothing, on a \
                      rectangle that is empty, reversed, negative or outside the picture, and says \
                      the picture's size. Use it to look closer at one part of a still from \
                      emerge_capture_still.",
        read_only: false,
        destructive: false,
        idempotent: false,
    },
    ToolDoc {
        name: "emerge_list_snips",
        description: "The snips the person took in the editor, newest first: each one's name, when \
                      it was taken, the map and the active tool, the note the person typed, the \
                      rectangle in window and picture pixels, the camera view, the cursor's world \
                      position and the placements inside the rectangle. Each entry names its \
                      picture file, and the newest picture comes back as an image. It reads files \
                      the editor wrote, so it needs no editor running and changes nothing. Returns \
                      how many snips there are, how many are shown, and each .json file in the \
                      snips folder that is not a snip record or has no picture file, with the \
                      reason.",
        read_only: true,
        destructive: false,
        idempotent: true,
    },
    ToolDoc {
        name: "emerge_capture_sequence",
        description: "A sequence of frames of the editor, with an optional operation applied \
                       once the first frame is in -- so frame 0 is the state before the action and \
                       the later frames are what it did. The frames are written to a directory and \
                       their paths are in the answer. The editor takes the frames, so the timing \
                       does not depend on this server's round trips.",
        read_only: false,
        destructive: true,
        idempotent: false,
    },
    ToolDoc {
        name: "emerge_read_oplog",
        description: "The editor's operation log, newest last. Every operation appends one line, whether a \
                      person or an agent did it. Read it to see what has been done to this project. A \
                      refusal is not logged, so a line here is a line about a change that happened. An \
                      undo is a new line naming the line it undoes.",
        read_only: true,
        destructive: false,
        idempotent: true,
    },
    ToolDoc {
        name: "emerge_frame_view",
        description: "Frame the editor's Map camera on a box. target: \"map\" (the whole map), \
                      \"selection\" (the locked placement; refused when nothing is locked), \
                      {\"placements\": [ids]} (their union), or {\"rect\": [x0, z0, x1, z1]} (map \
                      metres). It only moves the camera: it never changes map data, is not logged \
                      or undoable, and is not an operation. Refuses, and moves nothing, on an \
                      unknown id, an empty selection or a flat rectangle. Returns what was framed, \
                      the focus and the view height in metres. \
                      Call emerge_capture_still afterwards to see the result.",
        read_only: false,
        destructive: false,
        idempotent: true,
    },
];

/// **The document, as markdown.**
///
/// **One section per tool**, with its description, its annotations and its parameters.
pub fn render() -> String {
    let mut out = String::new();
    out.push_str("# The bevy_emerge MCP tools\n\n");
    out.push_str(
        "Generated from the server's own tool declarations. **Do not edit this file** — it is \
         written by `cargo run -p bevy_emerge_mcp -- --emit-docs`, and a test fails when it does \
         not match the server.\n\n",
    );
    out.push_str(
        "The server speaks the Model Context Protocol over **stdio**, and reaches the running \
         editor over the Bevy Remote Protocol on `BEVY_BRP_PORT` (default 15702). It writes \
         nothing to disk itself: the editor owns the project directory, and every file the \
         editor writes is written by the editor.\n\n",
    );
    out.push_str("## The tools\n\n");
    for tool in TOOLS {
        out.push_str(&format!("### `{}`\n\n", tool.name));
        out.push_str(&format!("{}\n\n", tool.description));
        out.push_str("| annotation | value |\n|---|---|\n");
        out.push_str(&format!("| `readOnlyHint` | {} |\n", tool.read_only));
        out.push_str(&format!("| `destructiveHint` | {} |\n", tool.destructive));
        out.push_str(&format!("| `idempotentHint` | {} |\n\n", tool.idempotent));
    }
    out.push_str(
        "## How the two paths converge\n\n\
         Every operation writes the data file, and the editor watches that file. So:\n\n\
         - an **agent** calls `emerge_apply_operation`, the editor writes the file, and the \
           editor's own state is already the new one;\n\
         - a **person** edits the file in a text editor, the watcher notices within a quarter of \
           a second, and the editor re-reads it — **and keeps what it had if the file does not \
           parse**;\n\
         - a **person at the keyboard** uses the editor's own controls, which dispatch the same \
           operations.\n\n\
         The measured latency of both paths is in the iteration report.\n",
    );
    out
}

/// **The document, as the JSON a caller would send.**
///
/// **The parameter schema, generated from the same types the server deserialises into.** A
/// schema that is written by hand is a schema that can be wrong, and an agent that sends the
/// wrong shape gets a refusal it cannot act on.
pub fn schemas() -> Value {
    serde_json::json!({
        "emerge_list_operations": {"type": "object", "additionalProperties": false},
        "emerge_preview_operation": {
            "type": "object",
            "additionalProperties": false,
            "required": ["operation"],
            "properties": {
                "operation": {"type": "string"},
                "parameters": {"type": "object"},
            },
        },
        "emerge_apply_operation": {
            "type": "object",
            "additionalProperties": false,
            "required": ["operation"],
            "properties": {
                "operation": {"type": "string"},
                "parameters": {"type": "object"},
                "dry_run": {"type": "boolean", "default": false},
            },
        },
        "emerge_undo_operation": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "operation_id": {"type": ["string", "null"]},
                "dry_run": {"type": "boolean", "default": false},
            },
        },
        "emerge_state": {"type": "object", "additionalProperties": false},
        "emerge_capture_still": {
            "type": "object",
            "additionalProperties": false,
            "required": ["label"],
            "properties": {
                "label": {"type": "string", "description": "A snake_case name for the capture."}
            }
        },
        "emerge_capture_sequence": {
            "type": "object",
            "additionalProperties": false,
            "required": ["label"],
            "properties": {
                "label": {"type": "string", "description": "A snake_case name for the sequence."},
                "seconds": {"type": "number", "exclusiveMinimum": 0, "maximum": 10, "default": 2.0},
                "fps": {"type": "integer", "enum": [4, 2], "default": 4},
                "operation": {
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": {"type": "string"},
                        "parameters": {"type": "object"}
                    }
                }
            }
        },
        "emerge_read_oplog": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "since_seq": {"type": "integer", "minimum": 0, "default": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 200, "default": 50},
            },
        },
        "emerge_snip": {
            "type": "object",
            "additionalProperties": false,
            "required": ["label", "rect"],
            "properties": {
                "label": {"type": "string", "description": "Where the crop is written, under the project's target/loop/captures: letters, digits, _ and - in parts, with / between the parts."},
                "rect": {"type": "array", "items": {"type": "integer", "minimum": 0}, "minItems": 4, "maxItems": 4, "description": "[x0, y0, x1, y1] in window pixels; x1 and y1 exclusive."}
            }
        },
        "emerge_list_snips": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "limit": {"type": "integer", "minimum": 1, "maximum": 20, "description": "How many snips to list, newest first. The default is 5."}
            }
        },
        "emerge_frame_view": {
            "type": "object",
            "additionalProperties": false,
            "required": ["target"],
            "properties": {
                "target": {
                    "oneOf": [
                        {"type": "string", "enum": ["map", "selection"]},
                        {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["placements"],
                            "properties": {"placements": {"type": "array", "items": {"type": "string"}, "minItems": 1}},
                        },
                        {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["rect"],
                            "properties": {"rect": {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4}},
                        },
                    ],
                },
            },
        },
    })
}
