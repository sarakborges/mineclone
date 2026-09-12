use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    hud::HudSettings,
    localization::{ActiveLanguage, Language},
    world::render_distance::{DEFAULT_RENDER_DISTANCE_CHUNKS, RenderDistanceSettings},
};

const CONFIG_FILE_NAME: &str = "config.json";
const CONFIG_DIRECTORY_NAME: &str = "Asteria";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct GameConfig {
    language: Language,
    graphics: GraphicsConfig,
    miscellaneous: MiscellaneousConfig,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            language: Language::default(),
            graphics: GraphicsConfig::default(),
            miscellaneous: MiscellaneousConfig::default(),
        }
    }
}

impl GameConfig {
    fn load() -> Self {
        let path = config_path();
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) if error.kind() == ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                eprintln!(
                    "failed to read game config {}: {error}; using defaults",
                    path.display()
                );
                return Self::default();
            }
        };

        serde_json::from_str(&source).unwrap_or_else(|error| {
            eprintln!(
                "failed to parse game config {}: {error}; using defaults",
                path.display()
            );
            Self::default()
        })
    }

    fn from_resources(
        language: &ActiveLanguage,
        hud: &HudSettings,
        render_distance: &RenderDistanceSettings,
    ) -> Self {
        Self {
            language: language.get(),
            graphics: GraphicsConfig {
                render_distance_chunks: render_distance.chunks(),
            },
            miscellaneous: MiscellaneousConfig {
                display_tooltips: hud.display_tooltips(),
            },
        }
    }

    fn save(&self) {
        let path = config_path();
        let Some(parent) = path.parent() else {
            eprintln!("game config path has no parent: {}", path.display());
            return;
        };

        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!(
                "failed to create game config directory {}: {error}",
                parent.display()
            );
            return;
        }

        let source = match serde_json::to_string_pretty(self) {
            Ok(source) => format!("{source}\n"),
            Err(error) => {
                eprintln!("failed to serialize game config: {error}");
                return;
            }
        };

        if fs::read_to_string(&path)
            .is_ok_and(|current| current == source)
        {
            return;
        }

        if let Err(error) = fs::write(&path, source) {
            eprintln!("failed to write game config {}: {error}", path.display());
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct MiscellaneousConfig {
    display_tooltips: bool,
}

impl Default for MiscellaneousConfig {
    fn default() -> Self {
        Self {
            display_tooltips: true,
        }
    }
}

pub(crate) struct GameConfigPlugin;

impl Plugin for GameConfigPlugin {
    fn build(&self, app: &mut App) {
        let config = GameConfig::load();

        let mut active_language = ActiveLanguage::default();
        active_language.set(config.language);

        let mut hud_settings = HudSettings::default();
        hud_settings.set_display_tooltips(config.miscellaneous.display_tooltips);

        let mut render_distance = RenderDistanceSettings::default();
        render_distance.set_chunks(config.graphics.render_distance_chunks);

        app.insert_resource(active_language)
            .insert_resource(hud_settings)
            .insert_resource(render_distance)
            .add_systems(Last, persist_game_config);
    }
}

fn persist_game_config(
    language: Res<ActiveLanguage>,
    hud: Res<HudSettings>,
    render_distance: Res<RenderDistanceSettings>,
) {
    if !language.is_changed() && !hud.is_changed() && !render_distance.is_changed() {
        return;
    }

    GameConfig::from_resources(&language, &hud, &render_distance).save();
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
