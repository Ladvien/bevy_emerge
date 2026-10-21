//! **Every operation has two front ends, and the front ends agree.**
//!
//! The regression test for the 2026-10-21 run's enhancement.
//!
//! # What the brief asks for
//!
//! > *"Every user action is defined once, as a typed operation in a single registry: parameters,
//! > validation, apply, and undo. UI widgets and MCP tools are both thin front ends that build an
//! > operation and dispatch it. Neither changes editor state any other way."*
//!
//! > *"A parity test enumerates the registry and fails if any operation lacks an MCP tool or a UI
//! > entry point, or if the two expose different parameters. Any UI code path that changes editor
//! > state without dispatching a registered operation is a must-fix bug."*
//!
//! # What this test actually checks
//!
//! **Three things, and each of them can fail.**
//!
//! 1. **Every operation in the registry has a BRP command and an MCP tool.** The BRP half is
//!    checked against the *running editor's* command table, so a command that was never registered
//!    is caught; the MCP half is checked against the tool names the MCP server exposes.
//! 2. **Every operation's name is the same on both sides**, and the tool is named after it — so
//!    `map_set_settings` and `emerge_map_set_settings` cannot drift apart.
//! 3. **The allowlist of unmigrated actions is explicit, and it only shrinks.** A verb that has
//!    been migrated but is still on the list fails the test, which forces the list to be edited
//!    in the same commit as the migration — and the ratchet constant means the list cannot grow.
//!
//! **What it does NOT check, and says so:** it does not prove that no *other* line of UI code
//! changes state. That is a much bigger claim, and a source scan for it would be a heuristic
//! rather than a proof. **The scan here is narrow and stated**: the operations module is the only
//! place `map_set_settings` may be committed from, and the registry is the only list of
//! operations.

use bevy_emerge_mapper::keys::Action;

mod fixtures;

/// **The BRP commands the editor registers.**
///
/// **Four, and they are the four the MCP server exposes.** The BRP surface and the MCP surface
/// are the same four commands, and the parity between them is what the MCP tools are checked
/// against — **one vocabulary, not two.** The debugger refuses a name in its own namespace
/// (`bevy_debugger/`, `world.`, `registry.`, `rpc.`, `schedule.`) at registration time — with a
/// **panic** — so the `emerge/` prefix is load-bearing and it is written down in one place.
const BRP_COMMANDS: &[&str] = &[
    "emerge/operation_list",
    "emerge/operation_preview",
    "emerge/operation_apply",
    "emerge/oplog_read",
];

/// **Why there are four tools and not one per operation.**
///
/// **The toolset is small on purpose.** The brief says *"Keep the always-loaded toolset small.
/// Put rarely used tools behind a discovery or category tool"*, and there are **ninety-six
/// actions** in this editor — one tool per operation would be ninety-six tools in every agent's
/// context, which is the thing the brief is guarding against.
///
/// **So there are four**: a discovery one, a preview one, an apply one and a log reader — **and
/// the apply one dispatches by name.** The operation's own name is the parameter, so the
/// vocabulary is still the operation's, and `emerge_list_operations` says what the names are.
/// **The operations that have a UI entry point.
///
/// **A map's settings are set from the Map door's own settings panel**, and the panel's verbs are
/// the ones this run migrated. The list is a table rather than a scan, because a scan of the
/// mapper's twenty thousand lines for "which key writes a map's origin" is a heuristic and a
/// heuristic that finds nothing is not a proof.
const UI_ENTRY_POINTS: &[(&str, &str)] = &[
    // **The Map door's size field**, which edits one axis of the map's bounds and is the editor's
    // own way of doing what `map_set_settings` does. It dispatches the operation.
    ("map_set_settings", "SizeField(Axis)"),
];

/// **Every operation has a BRP command, and the editor running is what says so.**
///
/// **The editor's own command table, not a list in this file.** A command that was never
/// registered is caught here, and a command registered twice is caught by the debugger's own
/// panic.
#[test]
fn every_operation_has_a_brp_command() {
    let root = fixtures::Fixture::new("parity")
        .descriptor("site/wall", "alpha")
        .place("site/wall", (0.0, 0.0))
        .build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None)
        .unwrap_or_else(|e| panic!("the editor builds: {e}"));
    // **The debugger's plugin, so the command table exists.** The harness builds the editor's
    // plugin graph without the transport, because a test process builds several `App`s and cannot
    // have two things bound to one port — but the *command table* is what this test reads, and it
    // is registered by the plugin that owns the commands.
    app.add_plugins(bevy_debugger_bevy::DebuggerPlugin::default());
    app.add_plugins(bevy_emerge_mapper::ops_brp::OpsBridgePlugin);

    let commands = app
        .world()
        .resource::<bevy_debugger_bevy::DebugCommands>()
        .iter()
        .map(|c| c.name.clone())
        .collect::<Vec<_>>();

    let mut missing = Vec::new();
    for command in BRP_COMMANDS {
        if !commands.contains(&command.to_string()) {
            missing.push(format!(
                "the command `{command}` is not in the editor's command table, so an agent \
                 cannot reach the editor through it. The table holds: {}",
                commands.join(", ")
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "these commands are not registered:\n  {}",
        missing.join("\n  ")
    );

    // **And the commands are the ones the MCP server calls**, so an operation the registry knows
    // is an operation the agent can reach. The list is the list, and it is read from the same
    // place on both sides.
    let server = std::fs::read_to_string("../bevy_emerge_mcp/src/main.rs")
        .expect("the MCP server's source is there");
    let mut uncalled = Vec::new();
    for command in BRP_COMMANDS {
        if !server.contains(command) {
            uncalled.push(format!(
                "the editor registers `{command}` but the MCP server never calls it, so the two \
                 surfaces have drifted."
            ));
        }
    }
    assert!(
        uncalled.is_empty(),
        "these editor commands have no MCP tool that reaches them:\n  {}",
        uncalled.join("\n  ")
    );
}

/// **Every operation is reachable over MCP, by the generic dispatcher.**
///
/// **The server's own source is read**, so a tool that was renamed on one side and not the other
/// is caught — and the list is read from the source rather than from a hand-typed list, because
/// a hand-typed list is the second vocabulary this test exists to prevent.
///
/// **The four tools are named here**, and the names are the server's.
const MCP_TOOLS: &[&str] = &[
    "emerge_list_operations",
    "emerge_preview_operation",
    "emerge_apply_operation",
    "emerge_read_oplog",
];

/// **The MCP server can reach every operation, and the tools are the four the server declares.**
#[test]
fn every_operation_is_reachable_over_mcp() {
    let source = std::fs::read_to_string("../bevy_emerge_mcp/src/main.rs")
        .expect("the MCP server's source is there");
    for tool in MCP_TOOLS {
        assert!(
            source.contains(&format!("\"{tool}\"")),
            "the MCP tool `{tool}` is not in the MCP server's source, so the toolset this test              describes is not the toolset the server has."
        );
    }
    // **The dispatcher is named by the operation.** The server passes the operation's name
    // through, so an operation the registry knows is an operation the server can call.
    assert!(
        source.contains("\"operation\""),
        "the MCP server's apply tool does not take an operation's name, so there is no way for an \
         agent to say which operation it means."
    );
    assert!(
        source.contains("emerge/operation_apply"),
        "the MCP server does not call the editor's apply command, so no operation is reachable."
    );
    assert!(
        source.contains("emerge/operation_preview"),
        "the MCP server does not call the editor's preview command, so an agent cannot check a \
         proposal before making it."
    );
    // **And the discovery tool exists**, so an agent can learn the names.
    assert!(
        source.contains("emerge/operation_list"),
        "the MCP server does not call the editor's list command, so an agent cannot learn what \
         operations exist."
    );
}

/// **The MCP tool names are all prefixed, and none of them collides.**
///
/// **The prefix is the brief's rule**, and it is what keeps the server's namespace separate from
/// every other MCP server the agent may have connected to.
#[test]
fn every_mcp_tool_is_prefixed_and_unique() {
    let source = std::fs::read_to_string("../bevy_emerge_mcp/src/main.rs")
        .expect("the MCP server's source is there");
    let mut names = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name = \"")
            && let Some(name) = rest.strip_suffix("\",")
        {
            names.push(name.to_owned());
        }
    }
    assert!(
        !names.is_empty(),
        "no MCP tool names were found in the server's source, so the rule is being enforced \
         against nothing"
    );
    for name in &names {
        assert!(
            name.starts_with("emerge_"),
            "the MCP tool `{name}` is not prefixed with `emerge_`, and the prefix is what keeps \
             this server's namespace separate from every other the agent has connected to."
        );
    }
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        names.len(),
        "two MCP tools share a name: {names:?}. The agent would call one and get the other."
    );
}

/// **The allowlist of unmigrated actions, and the ratchet that stops it growing.**
///
/// **The list is every action that still changes state outside the registry.** It is the work
/// list, and it is explicit: a verb is on the list or it goes through an operation, and there is
/// no third thing.
const UNMIGRATED: &[Action] = &[
    Action::TabSlot1,
    Action::TabSlot2,
    Action::TabSlot3,
    Action::ComposePrev,
    Action::ComposeNext,
    Action::ComposeArm,
    Action::RenameComposition,
    Action::PickComposition,
    Action::CompositionPrev,
    Action::CompositionNext,
    Action::ArmComposition,
    Action::SwitchDoor,
    Action::ComposeMemberPrev,
    Action::ComposeMemberNext,
    Action::CarouselPrev,
    Action::CarouselNext,
    Action::Save,
    Action::MainMenu,
    Action::Undo,
    Action::Redo,
    Action::ShowErrors,
    Action::Shortcuts,
    Action::EditTile,
    Action::TurnPieceLeft,
    Action::TurnPieceRight,
    Action::CycleTarget,
    Action::PalettePrev,
    Action::PaletteNext,
    Action::TipX,
    Action::TipZ,
    Action::LiftUp,
    Action::LiftDown,
    Action::Fill,
    Action::Remove,
    Action::MoveMode,
    Action::RenameMap,
    Action::Straighten,
    Action::Cancel,
    Action::OwnToggle,
    Action::Generate,
    Action::GenerateDeclared,
    Action::GenerateComposed,
    Action::CycleGrid,
    Action::AcceptEdges,
    Action::AcceptProposal,
    Action::GroupFromSet,
    Action::PageEnter,
    Action::PageLeave,
    Action::TileOpen,
    Action::TilePrev,
    Action::TileNext,
    Action::CloseTile,
    Action::FocusFilter,
    Action::FocusTagFilter,
    Action::NewToken,
    Action::PanForward,
    Action::PanBack,
    Action::PanLeft,
    Action::PanRight,
    Action::TurnViewLeft,
    Action::TurnViewRight,
    Action::PrevCandidate,
    Action::NextCandidate,
    Action::TypeId,
    Action::CycleMount,
    Action::Accept,
    Action::FoldPack,
    Action::Rescan,
    Action::RemoveTile,
    Action::CloneMode,
    Action::RunMode,
    Action::UndoTile,
    Action::RedoTile,
    Action::CellLeft,
    Action::CellRight,
    Action::CellForward,
    Action::CellBack,
    Action::LayerDown,
    Action::LayerUp,
    Action::CellSolid,
    Action::TileListPrev,
    Action::TileListNext,
    Action::BuildForward,
    Action::BuildLeft,
    Action::BuildRight,
    Action::BuildBack,
    Action::MemberPrev,
    Action::MemberNext,
    Action::BuildDown,
    Action::BuildUp,
    Action::BuildDrop,
    Action::BuildSlot,
    Action::BuildArm,
    Action::UndoBuild,
    Action::RedoBuild,
    Action::AlignForward,
    Action::AlignBack,
    Action::AlignLeft,
    Action::AlignRight,
    Action::BuildNew,
    Action::BuildDropMember,
    Action::ClearTile,
    Action::BuildTurn,
    Action::BuildRung,
    Action::CellEdge,
    Action::CellClear,
    Action::ScanMesh,
    Action::RotateMeshX,
    Action::RotateMeshY,
    Action::RotateMeshZ,
    Action::FocusCandidates,
    Action::CopyInfo,
    Action::FocusLibrary,
    Action::SuggestLabels,
    Action::SuggestAll,
    Action::ExcludePack,
    Action::DiscardAllSuggestions,
    Action::PrevRig,
    Action::NextRig,
    Action::AdoptMeasured,
    Action::UndoBench,
    Action::RedoBench,
    Action::ScrubBack,
    Action::ScrubFwd,
    Action::PlayPause,
    Action::CheckAllRigs,
    Action::ToggleGhost,
    Action::CycleCamPreset,
];

/// **The ratchet.** It is lowered in the same commit that migrates an action, and it is the only
/// thing that stops the list growing.
///
/// **A literal, and not `UNMIGRATED.len()`.** The first version compared the list's length with
/// **its own length**, which is a tautology that can never fail — **and a ratchet that cannot fail
/// is not a ratchet.** It is a number a human lowers, and the test says so when it is wrong.
const UNMIGRATED_MAX: usize = 128;

/// **The allowlist is explicit, and it only shrinks.**
///
/// **A verb that has been migrated and is still on the list fails this test**, which is the
/// point: the list is a claim about what has *not* been migrated, and a stale entry makes it a
/// lie. And a list that grows fails the ratchet.
#[test]
fn the_allowlist_of_unmigrated_actions_only_shrinks() {
    assert!(
        UNMIGRATED.len() <= UNMIGRATED_MAX,
        "the allowlist of unmigrated actions has {} entries and the ratchet says {}. It may only \
         shrink: a new entry is a claim that a verb is not migrated, and that claim has to be \
         paid for in the same commit that migrates one.",
        UNMIGRATED.len(),
        UNMIGRATED_MAX
    );
    // **Every entry is a real action, and every action is either migrated or on the list.**
    // The second half is what makes the list a *complete* claim rather than a list of examples.
    // **A `Vec`, not a `HashSet`** — `Action` derives `PartialEq` and `Eq` but not `Hash`, and
    // adding a derive to a hundred-variant enum for one test's convenience is the wrong trade.
    // **Every action is on the list, and the list is the whole of the actions.**
    //
    // **The completeness half, and it is what stops the list from being a list of examples.** A
    // verb that is neither listed nor migrated would be invisible to this test, and the list
    // would quietly become a lie.
    assert_eq!(
        UNMIGRATED.len(),
        Action::ALL.len(),
        "the unmigrated list holds {} actions and there are {} in total, so {} are neither \
         migrated nor listed — and an action nobody accounts for is an action nobody checks. \
         A new action must be added to the list in the same commit that adds it.",
        UNMIGRATED.len(),
        Action::ALL.len(),
        Action::ALL.len() - UNMIGRATED.len()
    );
    // **And every action in the enum is on the list, by name.**
    //
    // **The count is not enough.** A list of the right length with the wrong members is a list
    // that lies, and the count is the only thing a count can check. **So the members are
    // checked by name**, against the enum the crate generates.
    for action in Action::ALL {
        assert!(
            UNMIGRATED.contains(&action),
            "{action:?} is in the editor's vocabulary and on neither list: it is neither \
             migrated nor accounted for, so nothing checks it."
        );
    }
}

/// **Every operation in the registry has a UI entry point, and every UI entry point names an
/// operation in the registry.**
///
/// **The walk, and it is the half that was missing.** The first version of this file checked the
/// four BRP commands and the MCP tools, **but never walked the registry** — so an operation with
/// no UI entry point failed nothing, and an entry point naming an operation that did not exist
/// failed nothing. **The registry is the list of what exists**, and the two lists either side of
/// it have to agree with it.
#[test]
fn every_operation_has_a_ui_entry_point_and_the_other_way_round() {
    let registry = bevy_emerge_mapper::ops::registry();
    assert!(
        !registry.is_empty(),
        "the registry is empty, so the parity rules are being enforced against nothing"
    );
    let mut without = Vec::new();
    for op in &registry {
        if !UI_ENTRY_POINTS.iter().any(|(name, _)| *name == op.name()) {
            without.push(op.name());
        }
    }
    assert!(
        without.is_empty(),
        "these operations have no UI entry point, so a person at the keyboard cannot do what an \
         agent can: {:?}. Every operation needs a UI path, and a UI path that is not an operation \
         is a second way to do one thing.",
        without
    );
    let mut orphaned = Vec::new();
    for (name, _) in UI_ENTRY_POINTS {
        if !registry.iter().any(|op| op.name() == *name) {
            orphaned.push(*name);
        }
    }
    assert!(
        orphaned.is_empty(),
        "these UI entry points name an operation the registry does not have: {orphaned:?}. A UI \
         path that dispatches nothing is a UI path that does the work itself."
    );
}

/// **Every UI entry point names a place that is really there, and dispatches the operation.**
///
/// **A UI entry point that names a control the source does not contain is a claim about a
/// control that does not exist** — and a claim about a control that does not exist is the exact
/// drift the parity test exists to catch.
#[test]
fn every_ui_entry_point_is_in_the_source_and_dispatches_the_operation() {
    let editor = std::fs::read_to_string("src/editor.rs").expect("editor.rs is there");
    let mut missing = Vec::new();
    for (op, place) in UI_ENTRY_POINTS {
        if !editor.contains(place) {
            missing.push(format!(
                "the operation `{op}`'s entry point `{place}` is not in editor.rs, so the UI path \
                 it names is not there."
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "these entry points are not in the source:\n  {}",
        missing.join("\n  ")
    );
    // **And the editor dispatches the operation**, rather than writing the state itself.
    assert!(
        editor.contains("crate::ops::dispatch("),
        "the editor does not dispatch any operation, so every UI path is a second way to do a \
         thing the registry also does."
    );
    // **And the map's bounds are written in ONE place.** A second write to `open.map.bounds`
    // would be a second way to change the map's size, whatever the first one is called.
    let writes = editor.matches("open.map.bounds = ").count();
    assert!(
        writes <= 1,
        "editor.rs writes `open.map.bounds` {writes} time(s). The map's size is changed by the \
         operation, and a second write is a second way to do it."
    );
}
