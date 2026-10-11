//! **Import every `.glb` under a directory into a library file, without a window.**
//!
//! The editor's Tiles tab imports one mesh at a time and scans the whole asset root; a game with a
//! flat directory of a hundred first-party pieces (Sanctuary's `assets/models/kit`) wants them all
//! in one step, then labelled in the editor. This is the same `import::scan` the editor calls, and
//! each candidate's proposal is appended exactly as the editor's accept key would append it: the
//! measurement and nothing else. Tags are a judgement and stay empty, for the labeller.
//!
//! Meshes the library already holds are skipped (that is what `scan` does), a mesh that cannot be
//! measured is reported and left out, and the library is validated before it is written.
//!
//! **A piece too big for the editor's lattice is left out too, and named.** The editor draws a
//! lattice of cells for a selected piece and refuses to write a library holding one past
//! [`MAX_LATTICE_CELLS`] (`Library::validate_lattices`), so a single tree or landmark in the file
//! makes every later save in the editor fail with "NOT WRITTEN". Such pieces are scenery a game
//! places from code; list them in the kit's `project.ron` `exclude` so the editor does not offer
//! them either.
//!
//! Run: `cargo run -p bevy_emerge_core --example import_dir -- <asset-root> <dir> <library.ron> [face_bands]`
//! where `<dir>` is a directory under `<asset-root>`, for example
//! `-- /path/to/game/assets models/kit /path/to/game/assets/emerge/kit/library.ron`, and
//! `face_bands` is the project's `kits.ron` `lattice.face_bands` (default 1).

use std::path::PathBuf;

use bevy_emerge_core::descriptor::{Subgrid, divisions};
use bevy_emerge_core::import::{self, Severity};
use bevy_emerge_core::library::{Library, MAX_LATTICE_CELLS};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (asset_root, dir, library_path, face_bands) = match args.as_slice() {
        [a, d, l] => (a, d, l, 1),
        [a, d, l, f] => (a, d, l, f.parse::<u32>().map_err(|e| format!("face_bands `{f}`: {e}"))?),
        _ => {
            return Err(
                "usage: import_dir <asset-root> <dir under asset-root> <library.ron> [face_bands]"
                    .to_owned(),
            );
        }
    };
    let asset_root = PathBuf::from(asset_root);
    let dir = asset_root.join(dir);
    let library_path = PathBuf::from(library_path);

    let text = std::fs::read_to_string(&library_path)
        .map_err(|e| format!("{}: {e}", library_path.display()))?;
    let mut library = Library::parse(&text).map_err(|e| format!("{}: {e}", library_path.display()))?;

    let candidates = import::scan(&asset_root, &dir, &library)?;
    let mut added = 0usize;
    let mut warned = 0usize;
    for c in candidates {
        if c.blocked() {
            let why: Vec<&str> = c.findings.iter().map(|f| f.message.as_str()).collect();
            eprintln!("skipped {}: {}", c.mesh, why.join("; "));
            continue;
        }
        if c.worst() == Some(Severity::Warn) {
            warned += 1;
        }
        if let Ok(div) = divisions(&c.proposed, face_bands)
            && Subgrid::volume(div) > MAX_LATTICE_CELLS
        {
            println!(
                "too big for the editor ({}x{}x{} cells, past {MAX_LATTICE_CELLS}), exclude it: {}",
                div.0, div.1, div.2, c.mesh
            );
            continue;
        }
        if library.get(&c.proposed.id).is_some() {
            eprintln!("skipped {}: id `{}` is already in the library", c.mesh, c.proposed.id);
            continue;
        }
        library.descriptors.push(c.proposed);
        added += 1;
    }

    library.validate().map_err(|e| format!("not written, the result is invalid: {e}"))?;
    let out = library.to_ron()?;
    bevy_emerge_core::ron_surgery::save_atomic(&library_path, &out)?;
    println!(
        "imported {added} mesh(es) from {} into {} ({warned} with warnings), {} descriptor(s) in all",
        dir.display(),
        library_path.display(),
        library.descriptors.len()
    );
    Ok(())
}
