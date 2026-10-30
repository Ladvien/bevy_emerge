//! **The sculpted ground and the kept water, drawn in the viewport** (#99, #100).
//!
//! `terrain_stroke` and `water_run` write a heightfield and a water field into the project; until
//! now nothing showed them, so an author sculpted and ran water blind. This draws both from the
//! files, the same way a game would read them, and follows them when they change on disk (a stroke,
//! a run, an undo, an agent's edit): the files are polled twice a second and the meshes rebuilt only
//! when one's modification time moved.
//!
//! The ground sits [`crate::editor::ground_drop`] under the datum, like the bounds floor, so a floor
//! piece lying on flat ground is not z-fighting it. Water is a translucent quad per wet cell at
//! `ground + depth`.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy_emerge_core::heightfield::Heightfield;
use bevy_emerge_core::water::WaterField;

use crate::project::{OpenMap, Project};

/// Water shallower than this, metres, is not drawn: the simulation leaves a film on every wet slope.
pub const MIN_DRAWN_DEPTH_M: f32 = 0.02;

/// Water's tint: translucent, because the ground under it is part of the answer.
const WATER_TINT: [f32; 4] = [0.16, 0.42, 0.78, 0.62]; // CHROME-OK: world ink — a translucent water surface over the ground, not panel text

/// Which of the two drawn things an entity is.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerrainView {
    Ground,
    Water,
    /// One chunk of the carved ground (#101); drawn instead of [`TerrainView::Ground`] while the map has volume edits.
    Volume,
}

/// **The ground as a lit mesh**, one vertex per cell, tinted low (green) to high (tan) over the
/// field's own range so relief reads from above.
pub fn ground_mesh(ground: &Heightfield) -> Mesh {
    let (w, d) = (ground.width, ground.depth);
    let (lo, hi) = ground.range();
    let span = (hi - lo).max(1.0e-3);
    let at = |ix: usize, iz: usize| ground.heights[iz.min(d - 1) * w + ix.min(w - 1)];
    let mut positions = Vec::with_capacity(w * d);
    let mut normals = Vec::with_capacity(w * d);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(w * d);
    for iz in 0..d {
        for ix in 0..w {
            let h = at(ix, iz);
            positions.push([ground.origin.0 + ix as f32 * ground.cell_m, h, ground.origin.1 + iz as f32 * ground.cell_m]);
            let dx = (at(ix + 1, iz) - at(ix.saturating_sub(1), iz)) / (2.0 * ground.cell_m);
            let dz = (at(ix, iz + 1) - at(ix, iz.saturating_sub(1))) / (2.0 * ground.cell_m);
            normals.push(Vec3::new(-dx, 1.0, -dz).normalize().to_array());
            let t = ((h - lo) / span).clamp(0.0, 1.0);
            colors.push([0.30 + 0.38 * t, 0.46 + 0.14 * t, 0.24 + 0.22 * t, 1.0]); // CHROME-OK: world ink — relief tint
        }
    }
    let mut indices = Vec::with_capacity((w - 1) * (d - 1) * 6);
    for iz in 0..d - 1 {
        for ix in 0..w - 1 {
            let a = (iz * w + ix) as u32;
            let (b, c, e) = (a + 1, a + w as u32, a + w as u32 + 1);
            indices.extend_from_slice(&[a, c, b, b, c, e]);
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// **The kept water as one mesh**: a flat quad over each cell deeper than [`MIN_DRAWN_DEPTH_M`], at
/// the ground's height there plus the depth. `None` when nothing is wet, or when the water was run on
/// a ground of another shape than this one.
pub fn water_mesh(ground: &Heightfield, water: &WaterField) -> Option<Mesh> {
    if (water.width, water.depth) != (ground.width, ground.depth) || water.depth_m.len() != ground.heights.len() {
        return None;
    }
    let half = water.cell_m * 0.5;
    let (mut positions, mut indices) = (Vec::new(), Vec::new());
    for iz in 0..water.depth {
        for ix in 0..water.width {
            let i = iz * water.width + ix;
            if water.depth_m[i] <= MIN_DRAWN_DEPTH_M {
                continue;
            }
            let (x, z, y) = (water.origin.0 + ix as f32 * water.cell_m, water.origin.1 + iz as f32 * water.cell_m, ground.heights[i] + water.depth_m[i]);
            let base = positions.len() as u32;
            positions.extend_from_slice(&[[x - half, y, z - half], [x + half, y, z - half], [x - half, y, z + half], [x + half, y, z + half]]);
            indices.extend_from_slice(&[base, base + 2, base + 1, base + 1, base + 2, base + 3]);
        }
    }
    if positions.is_empty() {
        return None;
    }
    let n = positions.len();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![WATER_TINT; n]);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

/// What the drawn meshes were built from; a change in any of it rebuilds them.
#[derive(Default, PartialEq)]
pub struct Built {
    map_path: std::path::PathBuf,
    ground: Option<std::time::SystemTime>,
    water: Option<std::time::SystemTime>,
    volume: Option<std::time::SystemTime>,
    origin: [u32; 3],
    drop: u32,
}

fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// **Draw the open map's terrain and water**, and redraw when either file changes.
pub fn sync_terrain_view(
    time: Res<Time>,
    project: Res<Project>,
    open: Res<OpenMap>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: Local<(f32, Option<Built>, Vec<Entity>)>,
) {
    let (since, built, drawn) = &mut *state;
    *since += time.delta_secs();
    if built.is_some() && *since < 0.5 {
        return;
    }
    *since = 0.0;
    let name = open.map_path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".map.ron")).unwrap_or_default().to_owned();
    let ground_path = project.emerge_dir.join("terrain").join(format!("{name}.ron"));
    let water_path = project.emerge_dir.join("water").join(format!("{name}.field.ron"));
    let volume_path = project.emerge_dir.join("volume").join(format!("{name}.ron"));
    let drop = crate::editor::ground_drop(&project);
    let now = Built {
        map_path: open.map_path.clone(),
        ground: modified(&ground_path),
        water: modified(&water_path),
        volume: modified(&volume_path),
        origin: [open.map.origin.0.to_bits(), open.map.origin.1.to_bits(), open.map.origin.2.to_bits()],
        drop: drop.to_bits(),
    };
    if built.as_ref() == Some(&now) {
        return;
    }
    for e in drawn.drain(..) {
        commands.entity(e).despawn();
    }
    let ground: Option<Heightfield> = ground_of(&ground_path);
    if let Some(ground) = &ground {
        let at = Transform::from_xyz(open.map.origin.0, open.map.origin.1 - drop, open.map.origin.2);
        let carved = volume_chunks(&project, &open, ground);
        let material = materials.add(StandardMaterial { perceptual_roughness: 1.0, ..default() });
        if carved.is_empty() {
            drawn.push(commands.spawn((TerrainView::Ground, Mesh3d(meshes.add(ground_mesh(ground))), MeshMaterial3d(material), at)).id());
        } else {
            for mesh in carved {
                drawn.push(commands.spawn((TerrainView::Volume, Mesh3d(meshes.add(mesh)), MeshMaterial3d(material.clone()), at)).id());
            }
        }
        let water: Option<WaterField> = std::fs::read_to_string(&water_path).ok().and_then(|t| ron::from_str(&t).ok());
        if let Some(mesh) = water.and_then(|w| water_mesh(ground, &w)) {
            drawn.push(commands.spawn((
                TerrainView::Water,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(StandardMaterial { alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.15, ..default() })),
                // The ground is dropped; the water is measured from the datum like pieces are.
                Transform::from_xyz(open.map.origin.0, open.map.origin.1, open.map.origin.2),
            )).id());
        }
    }
    *built = Some(now);
}

fn ground_of(path: &std::path::Path) -> Option<Heightfield> {
    ron::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// **The carved ground's chunk meshes** (#101): empty when the map has no volume edits, or when the
/// `isomesh` feature is off, and the heightfield mesh is drawn instead.
#[cfg(feature = "isomesh")]
fn volume_chunks(project: &Project, open: &OpenMap, ground: &Heightfield) -> Vec<Mesh> {
    let Some(volume) = crate::ops_volume::volume_of(project, open) else { return Vec::new() };
    let buffers = crate::volume_mesh::mesh_volume(&volume, ground).unwrap_or_default();
    buffers
        .into_iter()
        .map(|b| {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, b.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, b.normals);
            mesh.insert_indices(Indices::U32(b.indices));
            mesh
        })
        .collect()
}

#[cfg(not(feature = "isomesh"))]
fn volume_chunks(_: &Project, _: &OpenMap, _: &Heightfield) -> Vec<Mesh> {
    Vec::new()
}
