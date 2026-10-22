//! **The bevy_emerge MCP server**, as a library.
//!
//! The binary is the server; this is the half the tests read. **The tool declarations live here**
//! so that a test can check the reference against them without parsing source text — a test that
//! parses source text is a test of the source text, and it passes when the text is right and the
//! server is wrong.
//!
//! # What it is
//!
//! An MCP server for the [bevy_emerge](https://github.com/Ladvien/bevy_emerge) world editor. It
//! speaks the Model Context Protocol over **stdio** and reaches the running editor over the Bevy
//! Remote Protocol, so an agent can drive the editor through typed operations instead of
//! synthesising keystrokes.

pub mod brp;
pub mod docs;
pub mod snip;

/// **The version, for the server's own handshake.**
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
