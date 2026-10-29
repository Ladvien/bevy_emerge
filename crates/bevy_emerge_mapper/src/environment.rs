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

/// **One button of the Environment panel** (#98). The clock buttons are camera-style state
/// ([`ClockScrub`]); the weather buttons are `family_set` operations, one undo step each.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum EnvAction {
    /// Hold the preview's clock at this hour of day.
    Scrub(f64),
    /// Let the settings file's clock run again.
    ReleaseClock,
    /// Force this weather preset (`forced_weather`).
    Force(WeatherPreset),
    /// Return the weather to the climate's own schedule.
    ReleaseWeather,
}

impl EnvAction {
    /// Hours the scrubber offers: dawn, noon, dusk and night.
    pub const HOURS: [(f64, &'static str); 4] = [(6.0, "6:00"), (12.0, "12:00"), (18.5, "18:30"), (23.0, "23:00")];

    /// The `family_set` params this action writes, or `None` for the camera-style clock buttons.
    pub fn family_params(self) -> Option<serde_json::Value> {
        let value = match self {
            EnvAction::Force(p) => format!("Some({p:?})"),
            EnvAction::ReleaseWeather => "None".to_owned(),
            EnvAction::Scrub(_) | EnvAction::ReleaseClock => return None,
        };
        Some(serde_json::json!({"crate": CRATE, "field": "forced_weather", "value": value}))
    }
}

/// The ENVIRONMENT section of the controls panel.
pub fn spawn_section(s: &mut ChildSpawnerCommands) {
    use crate::chrome::{GAP_TIGHT, Severity, button, section};
    section(s, "ENVIRONMENT");
    let row = |s: &mut ChildSpawnerCommands, items: &mut dyn FnMut(&mut ChildSpawnerCommands)| {
        s.spawn(Node { column_gap: Val::Px(GAP_TIGHT), flex_wrap: FlexWrap::Wrap, row_gap: Val::Px(GAP_TIGHT), ..default() })
            .with_children(|r| items(r));
    };
    row(s, &mut |r| {
        for (hour, label) in EnvAction::HOURS {
            button(r, EnvAction::Scrub(hour), label, Severity::Plain);
        }
        button(r, EnvAction::ReleaseClock, "Run", Severity::Plain);
    });
    row(s, &mut |r| {
        for (preset, label) in [(WeatherPreset::Clear, "Clear"), (WeatherPreset::Fog, "Fog"), (WeatherPreset::Rain, "Rain")] {
            button(r, EnvAction::Force(preset), label, Severity::Plain);
        }
        button(r, EnvAction::ReleaseWeather, "Release", Severity::Plain);
    });
}

/// **The preview's look** (#98): a project with a `family/bevy_elements.ron` gets `bevy_elements`'
/// atmosphere, fog, clouds and sun on the editor camera, which is what the game draws; a project
/// without one keeps the editor's own light. Follows the file's presence, so adding or deleting it
/// switches the preview without a restart.
pub fn dress_camera(
    settings: Res<FamilySettings>,
    mut camera: Query<(Entity, &mut Camera, Has<bevy_elements::ElementsCamera>), With<crate::view::MainCamera>>,
    mut commands: Commands,
) {
    let wanted = settings.text.contains_key(CRATE);
    for (entity, mut cam, has) in &mut camera {
        match (wanted, has) {
            (true, false) => {
                // `ElementsCamera` requires `Hdr`, which gives the camera a main texture of its own
                // that the final pass copies onto the surface. The default copy ignores what the
                // ground pass already drew there (`blend_state: None`), so a transparent main
                // texture copied that way blacked the whole viewport (#118). Blend it over instead.
                cam.clear_color = bevy::camera::ClearColorConfig::Custom(Color::NONE);
                cam.output_mode = bevy::camera::CameraOutputMode::Write {
                    blend_state: Some(bevy::render::render_resource::BlendState::ALPHA_BLENDING),
                    clear_color: bevy::camera::ClearColorConfig::None,
                };
                let f: u32 = std::env::var("W4F").ok().and_then(|v| v.parse().ok()).unwrap_or(63);
                commands.entity(entity).insert(bevy_elements::ElementsCamera { auto_exposure: f & 1 != 0, atmosphere: f & 2 != 0, distance_fog: f & 4 != 0, volumetric_fog: f & 8 != 0, ambient: f & 16 != 0, msaa_off: f & 32 != 0 });
            }
            (false, true) => {
                cam.clear_color = bevy::camera::ClearColorConfig::None;
                cam.output_mode = bevy::camera::CameraOutputMode::default();
                commands.entity(entity).remove::<bevy_elements::ElementsCamera>();
            }
            _ => {}
        }
    }
}
