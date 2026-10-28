//! **A running game follows its project's files.** [`EmergeLivePlugin`] watches the project
//! directory, and about a second after the last write reloads it and respawns *only* the placements
//! that changed. An unchanged placement keeps its `Entity`, so whatever a game hung on it (an agent's
//! booking, a camera target, an animation) survives an edit to the next table over.
//!
//! The reload is `bevy_emerge_core::project_data::load` plus `EmergeWorld::with_compositions`, the
//! same two doors a game's first load goes through. A reload either of them refuses leaves every
//! entity and the [`EmergeWorld`] as they were and writes no [`ProjectReloaded`].

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use bevy::prelude::*;
use bevy_emerge_core::live::{FileWatch, MapDiff, Verdict, diff_maps};
use bevy_emerge_core::map::Map;
use bevy_emerge_core::project_data::ProjectData;

use crate::{EmergeWorld, Placement, spawn_placements};

/// Watches `dir` (the project directory, the one holding `kits.ron`) and keeps the world in step.
///
/// `maps` is every map the game reads; [`ProjectReloaded::diffs`] has one entry per name. The map in
/// the [`EmergeWorld`] resource is the one that is respawned, and is read from the files by its own
/// name, so it need not be listed.
pub struct EmergeLivePlugin {
    pub dir: PathBuf,
    pub maps: Vec<String>,
}

/// One successful reload.
#[derive(Message, Clone, Debug)]
pub struct ProjectReloaded {
    /// What changed in each map, by name, expanded maps compared (a stamp's rows count).
    pub diffs: BTreeMap<String, MapDiff>,
    /// A descriptor (or its resolved masks) differs: every placement of it was respawned.
    pub library_changed: bool,
}

#[derive(Resource)]
struct Live {
    dir: PathBuf,
    maps: Vec<String>,
    watch: FileWatch,
    /// The last good version of every listed map, the baseline the next diff is against.
    last: BTreeMap<String, Map>,
}

impl Plugin for EmergeLivePlugin {
    fn build(&self, app: &mut App) {
        // Stat BEFORE loading: a write between the two is then a change the first poll sees.
        let watch = FileWatch::new(&self.dir, &["ron"]);
        let names: Vec<&str> = self.maps.iter().map(String::as_str).collect();
        let last = match ProjectData::load(&self.dir, &names) {
            Ok(data) => data.maps,
            Err(e) => {
                warn!("emerge live: the project does not load yet, diffs start from nothing: {e}");
                BTreeMap::new()
            }
        };
        app.add_message::<ProjectReloaded>()
            .insert_resource(Live {
                dir: self.dir.clone(),
                maps: self.maps.clone(),
                watch,
                last,
            })
            .add_systems(Update, reload);
    }
}

/// What to do to the entities: despawn these ids' entities, then spawn these ids.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    pub despawn: BTreeSet<String>,
    pub spawn: BTreeSet<String>,
    pub library_changed: bool,
}

/// **Which placements must be drawn again to turn `old` into `new`.**
///
/// A placement is respawned when it is new, when its row differs, when its resolved height differs
/// (a table moved, so the lamp on it did not change but stands elsewhere), when its descriptor or the
/// descriptor's masks differ, or when what it rests on is respawned (a relationship points at an
/// entity, and a despawned host takes it away).
pub(crate) fn plan(old: &EmergeWorld, new: &EmergeWorld) -> Plan {
    let before: HashMap<&str, usize> = old
        .map
        .placements
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();
    let mut out = Plan::default();
    out.library_changed = old.library != new.library || old.masks != new.masks;
    for (i, p) in new.map.placements.iter().enumerate() {
        let same = before.get(p.id.as_str()).is_some_and(|&j| {
            old.map.placements[j] == *p
                && old.y.get(j) == new.y.get(i)
                && old.entry(&p.descriptor) == new.entry(&p.descriptor)
                && old.map.origin == new.map.origin
        });
        if !same {
            out.spawn.insert(p.id.clone());
        }
    }
    // Riders follow their host, transitively.
    loop {
        let before_len = out.spawn.len();
        for p in &new.map.placements {
            if p.on.as_ref().is_some_and(|h| out.spawn.contains(h)) {
                out.spawn.insert(p.id.clone());
            }
        }
        if out.spawn.len() == before_len {
            break;
        }
    }
    let now: BTreeSet<&str> = new.map.placements.iter().map(|p| p.id.as_str()).collect();
    for p in &old.map.placements {
        if !now.contains(p.id.as_str()) || out.spawn.contains(&p.id) {
            out.despawn.insert(p.id.clone());
        }
    }
    out
}

fn load_world(live: &Live, current: &EmergeWorld) -> Result<(EmergeWorld, BTreeMap<String, Map>), String> {
    let mut names: Vec<&str> = live.maps.iter().map(String::as_str).collect();
    if !names.contains(&current.map.name.as_str()) {
        names.push(current.map.name.as_str());
    }
    let mut data = ProjectData::load(&live.dir, &names)?;
    let map = data
        .maps
        .get(&current.map.name)
        .cloned()
        .ok_or_else(|| format!("map `{}` was not loaded", current.map.name))?;
    // The loader returns the expanded map with its stamps still listed; the world must not expand
    // them a second time.
    let mut map = map;
    map.stamps.clear();
    let world = EmergeWorld::with_compositions(
        data.bound.library.clone(),
        map,
        data.vocab.clone(),
        &[],
        data.kits.lattice,
    )?;
    data.maps.retain(|name, _| live.maps.contains(name));
    Ok((world, data.maps))
}

#[allow(clippy::too_many_arguments)]
fn reload(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<AssetServer>,
    world: Option<ResMut<EmergeWorld>>,
    mut live: ResMut<Live>,
    placed: Query<(Entity, &Placement)>,
    mut reloaded: MessageWriter<ProjectReloaded>,
) {
    if !live.watch.poll(time.elapsed_secs_f64()) {
        return;
    }
    let Some(mut world) = world else { return };
    let verdict = |ok: bool, message: String| {
        if let Err(e) = Verdict::now(ok, message).write(&live.dir) {
            warn!("emerge live: could not write the verdict: {e}");
        }
    };
    let (new, maps) = match load_world(&live, &world) {
        Ok(loaded) => loaded,
        Err(e) => {
            warn!("emerge live: reload refused, keeping the last good project: {e}");
            verdict(false, e);
            return;
        }
    };
    let plan = plan(&world, &new);

    // The world's own map is diffed against what is on screen; the others against their last load.
    let empty = Map::default();
    let mut diffs = BTreeMap::new();
    for (name, map) in &maps {
        let old = if *name == new.map.name { &world.map } else { live.last.get(name).unwrap_or(&empty) };
        diffs.insert(name.clone(), diff_maps(old, map));
    }
    if !diffs.contains_key(&new.map.name) {
        diffs.insert(new.map.name.clone(), diff_maps(&world.map, &new.map));
    }

    let mut kept: HashMap<String, Entity> = HashMap::new();
    for (entity, Placement(id)) in &placed {
        if plan.despawn.contains(id) {
            commands.entity(entity).despawn();
        } else {
            kept.insert(id.clone(), entity);
        }
    }
    // A placement the old world never drew (no mesh, no footprint) has no entity to despawn, and one
    // whose entity is missing for any other reason is simply drawn.
    let spawned = spawn_placements(&mut commands, &assets, &new, Some(&plan.spawn), &kept);

    let summary = diffs
        .iter()
        .map(|(name, d)| format!("{name}: {}", d.summary()))
        .collect::<Vec<_>>()
        .join("; ");
    info!("emerge live: reloaded ({summary}); respawned {spawned}");
    verdict(true, summary);
    *world = new;
    live.last = maps;
    reloaded.write(ProjectReloaded {
        diffs,
        library_changed: plan.library_changed,
    });
}
