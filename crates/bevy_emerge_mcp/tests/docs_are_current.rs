//! **The MCP tool reference matches the server.**
//!
//! The brief's rule: *"MCP tools and `docs/mcp-tools.md` are generated from the registry, or
//! mechanically checked against it."* **This is the checked half** — the document is rendered
//! from the server's own declarations and compared with the file in the repository.
//!
//! **A stale copy fails the test**, and the failure says how to fix it. A hand-written reference
//! is a second vocabulary: it says one thing and the server does another, and the reader — an
//! agent, mostly — believes the document.

/// **The document, as the repository holds it.**
const ON_DISK: &str = include_str!("../../../docs/mcp-tools.md");

#[test]
fn the_tool_reference_is_current() {
    let rendered = bevy_emerge_mcp::docs::render();
    assert_eq!(
        rendered, ON_DISK,
        "docs/mcp-tools.md does not match the server's own tool declarations. Regenerate it with \
         `cargo run -q -p bevy_emerge_mcp -- --emit-docs > docs/mcp-tools.md`."
    );
}

/// **The reference names every tool the server has, and no others.**
///
/// **The tool list is the server's**, so a tool that was added and not documented — or
/// documented and removed — is caught here rather than by a reader.
#[test]
fn the_reference_names_every_tool() {
    let rendered = bevy_emerge_mcp::docs::render();
    for tool in bevy_emerge_mcp::docs::TOOLS {
        assert!(
            rendered.contains(&format!("### `{}`", tool.name)),
            "the reference does not document the tool `{}`.",
            tool.name
        );
    }
}

/// **The reference's schemas cover every tool.**
#[test]
fn every_tool_has_a_schema() {
    let schemas = bevy_emerge_mcp::docs::schemas();
    let map = schemas.as_object().expect("the schemas are an object");
    for tool in bevy_emerge_mcp::docs::TOOLS {
        assert!(
            map.contains_key(tool.name),
            "the tool `{}` has no parameter schema, so an agent cannot tell what to send it.",
            tool.name
        );
    }
    assert_eq!(
        map.len(),
        bevy_emerge_mcp::docs::TOOLS.len(),
        "there are more schemas than tools, or more tools than schemas, so one of the lists is \
         stale."
    );
}

/// **Every tool is prefixed, and the prefix is what keeps the namespaces apart.**
#[test]
fn every_tool_is_prefixed() {
    for tool in bevy_emerge_mcp::docs::TOOLS {
        assert!(
            tool.name.starts_with("emerge_"),
            "the tool `{}` is not prefixed with `emerge_`.",
            tool.name
        );
    }
}
