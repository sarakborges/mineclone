use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    app::{
        crash_log::{log_system_event, log_system_warn},
        keybinds::Keybinds,
    },
    hud::HudSettings,
    localization::{ActiveLanguage, Language},
    world::render_distance::{DEFAULT_RENDER_DISTANCE_CHUNKS, RenderDistanceSettings},
};

const CONFIG_FILE_NAME: &str = "config.json";
const CONFIG_DIRECTORY_NAME: &str = "Asteria";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct GameConfig {
    language: Language,
    graphics: GraphicsConfig,
    miscellaneous: MiscellaneousConfig,
    keybinds: Keybinds,
}

impl GameConfig {
    fn load() -> Self {
        let path = config_path();
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                log_system_event(format!(
                    "config.load defaults path={} reason=not_found",
                    path.display()
                ));
                return Self::default();
            }
            Err(error) => {
                log_system_warn(format!(
                    "config.load defaults path={} reason=read_failed error={error}",
                    path.display()
                ));
                return Self::default();
            }
        };

        match serde_json::from_str(&source) {
            Ok(config) => {
                log_system_event(format!("config.load success path={}", path.display()));
                config
            }
            Err(error) => {
                log_system_warn(format!(
                    "config.load defaults path={} reason=parse_failed error={error}",
                    path.display()
                ));
                Self::default()
            }
        }
    }

    fn from_resources(
        language: &ActiveLanguage,
        hud: &HudSettings,
        render_distance: &RenderDistanceSettings,
        keybinds: &Keybinds,
    ) -> Self {
        Self {
            language: language.get(),
            graphics: GraphicsConfig {
                render_distance_chunks: render_distance.chunks(),
            },
            miscellaneous: MiscellaneousConfig { hud: *hud },
            keybinds: *keybinds,
        }
    }

    fn save(&self) {
        let path = config_path();
        let Some(parent) = path.parent() else {
            log_system_warn(format!(
                "config.save failed path={} reason=missing_parent",
                path.display()
            ));
            return;
        };

        if let Err(error) = fs::create_dir_all(parent) {
            log_system_warn(format!(
                "config.save failed path={} reason=create_directory_failed error={error}",
                path.display()
            ));
            return;
        }

        let source = match serde_json::to_string_pretty(self) {
            Ok(source) => format!("{source}\n"),
            Err(error) => {
                log_system_warn(format!(
                    "config.save failed path={} reason=serialize_failed error={error}",
                    path.display()
                ));
                return;
            }
        };

        if fs::read_to_string(&path).is_ok_and(|current| current == source) {
            return;
        }

        match fs::write(&path, source) {
            Ok(()) => log_system_event(format!("config.save success path={}", path.display())),
            Err(error) => log_system_warn(format!(
                "config.save failed path={} reason=write_failed error={error}",
                path.display()
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct GraphicsConfig {
    render_distance_chunks: i32,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            render_distance_chunks: DEFAULT_RENDER_DISTANCE_CHUNKS,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct MiscellaneousConfig {
    hud: HudSettings,
}

pub(crate) struct GameConfigPlugin;

impl Plugin for GameConfigPlugin {
    fn build(&self, app: &mut App) {
        let config = GameConfig::load();

        let mut active_language = ActiveLanguage::default();
        active_language.set(config.language);

        let hud_settings = config.miscellaneous.hud;

        let mut render_distance = RenderDistanceSettings::default();
        render_distance.set_chunks(config.graphics.render_distance_chunks);

        app.insert_resource(active_language)
            .insert_resource(hud_settings)
            .insert_resource(render_distance)
            .insert_resource(config.keybinds)
            .add_systems(Last, persist_game_config);
    }
}

fn persist_game_config(
    language: Res<ActiveLanguage>,
    hud: Res<HudSettings>,
    render_distance: Res<RenderDistanceSettings>,
    keybinds: Res<Keybinds>,
) {
    if !language.is_changed()
        && !hud.is_changed()
        && !render_distance.is_changed()
        && !keybinds.is_changed()
    {
        return;
    }

    GameConfig::from_resources(&language, &hud, &render_distance, &keybinds).save();
}

fn config_path() -> PathBuf {
    config_directory().join(CONFIG_FILE_NAME)
}

fn config_directory() -> PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(root) = env::var_os("APPDATA") {
        return PathBuf::from(root).join(CONFIG_DIRECTORY_NAME);
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(CONFIG_DIRECTORY_NAME);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(root).join(CONFIG_DIRECTORY_NAME);
    }

    fallback_config_directory()
}

fn fallback_config_directory() -> PathBuf {
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(format!(".{}", CONFIG_DIRECTORY_NAME.to_lowercase()));
    }

    env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join(format!(".{}", CONFIG_DIRECTORY_NAME.to_lowercase()))
}
