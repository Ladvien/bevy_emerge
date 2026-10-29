//! **A map's environment: climate, date, clock, place and forced weather** (#98).
//!
//! The data is `bevy_elements`' own serde types, written as RON in `<project>/family/bevy_elements.ron`
//! (the file `family_set` edits and the game's copy of the plugin reads). [`ElementsSettings`] only
//! lists which of the crate's resources the file holds; it defines no field of its own except which
//! weather preset is forced, because the crate keeps that inside a private director.
//!
//! [`apply_environment`] copies a changed file into the running resources, so the editor's preview
//! and the game see the same values. Forcing rain is `WeatherDirector::force`; clearing the field
//! releases the weather back to the climate's own schedule.

use std::time::Duration;

use bevy::prelude::*;
use bevy_elements::{Climate, GeoLocation, WeatherDirector, WeatherPreset, WorldClock};
use serde::{Deserialize, Serialize};

use crate::family_host::FamilySettings;

/// The crate this settings file belongs to, as `family.ron` names it.
pub const CRATE: &str = "bevy_elements";

/// Game time a forced or released weather takes to blend, so the sky does not jump.
const BLEND: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementsSettings {
    pub clock: WorldClock,
    #[serde(default)]
    pub location: GeoLocation,
    pub climate: Climate,
    /// `Some(Rain)` forces rain; `None` (the default) lets the climate's scheduler choose.
    #[serde(default)]
    pub forced_weather: Option<WeatherPreset>,
}

impl ElementsSettings {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("bevy_elements settings: {e}"))
    }

    /// The settings a new map starts with: the crate's own defaults.
    pub fn defaults() -> Self {
        ElementsSettings {
            clock: WorldClock::default(),
            location: GeoLocation::default(),
            climate: Climate::default(),
            forced_weather: None,
        }
    }

    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).expect("settings serialise")
    }
}

/// Puts `settings` into the running resources. `applied_force` is the forced preset this was last
/// called with, so an unchanged field is not forced again (which would restart the blend).
pub fn apply(
    settings: &ElementsSettings,
    clock: &mut WorldClock,
    location: &mut GeoLocation,
    climate: &mut Climate,
    director: &mut WeatherDirector,
    applied_force: &mut Option<Option<WeatherPreset>>,
) {
    *clock = settings.clock.clone();
    *location = settings.location.clone();
    *climate = settings.climate.clone();
    if *applied_force != Some(settings.forced_weather) {
        match settings.forced_weather {
            Some(preset) => director.force(preset, BLEND),
            None if director.is_forced() => director.release(BLEND),
            None => {}
        }
        *applied_force = Some(settings.forced_weather);
    }
}

/// The preview's resources the settings file drives; absent until `bevy_elements`' plugins are added.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Preview<'w> {
    clock: Option<ResMut<'w, WorldClock>>,
    location: Option<ResMut<'w, GeoLocation>>,
    climate: Option<ResMut<'w, Climate>>,
    director: Option<ResMut<'w, WeatherDirector>>,
}

/// Reads a reloaded `family/bevy_elements.ron` into the preview's resources. A file that does not
/// fit the crate's types is a problem on the status line and the previous values stay.
pub fn apply_environment(
    settings: Res<FamilySettings>,
    mut state: ResMut<crate::editor::EditorState>,
    mut preview: Preview,
    mut applied_force: Local<Option<Option<WeatherPreset>>>,
    mut seen: Local<Option<String>>,
) {
    let Some(text) = settings.text.get(CRATE) else {
        *seen = None;
        return;
    };
    if seen.as_ref() == Some(text) {
        return;
    }
    *seen = Some(text.clone());
    let parsed = match ElementsSettings::from_ron(text) {
        Ok(p) => p,
        Err(e) => {
            state.status.problem(format!("{e}; the previous environment stays"));
            return;
        }
    };
    if let (Some(clock), Some(location), Some(climate), Some(director)) =
        (preview.clock.as_mut(), preview.location.as_mut(), preview.climate.as_mut(), preview.director.as_mut())
    {
        apply(&parsed, clock, location, climate, director, &mut applied_force);
    }
}

/// **The clock scrubber** (#98): camera-style state, not an operation. `Some(hour)` holds the
/// preview's clock at that hour of day without touching `family/bevy_elements.ron`, so scrubbing
/// writes no file and leaves no undo step; `None` lets the file's clock run.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct ClockScrub(pub Option<f64>);

impl ClockScrub {
    /// Hold the clock at `hour`, wrapped into `[0, 24)`. A non-finite hour is ignored.
    pub fn set(&mut self, hour: f64) {
        if hour.is_finite() {
            self.0 = Some(hour.rem_euclid(24.0));
        }
    }

    /// Give the clock back to the settings file.
    pub fn release(&mut self) {
        self.0 = None;
    }
}

/// Holds the preview's clock at the scrubbed hour, every frame after the file was applied.
pub fn scrub_clock(scrub: Res<ClockScrub>, clock: Option<ResMut<WorldClock>>) {
    if let (Some(hour), Some(mut clock)) = (scrub.0, clock)
        && (clock.hour != hour || !clock.paused)
    {
        clock.hour = hour;
        clock.paused = true;
    }
}
