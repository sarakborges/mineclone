use std::{collections::BTreeMap, fs, path::PathBuf};

use bevy::prelude::*;
use chrono::Local;
use serde::Serialize;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{block_orientation::BlockOrientation, builtin_ids::STRUCTURE_TOOL_ID},
    gameplay::availability::world_interaction_available,
    hud::chat::ChatState,
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    targeting::{ToolUse, ToolUseButton, block::{BlockTargetingSet, TargetedBlock}, placement_voxel},
    voxel::world::VoxelWorld,
};

const SELECTION_COLOR: Color = Color::srgba(0.72, 0.58, 0.98, 0.98);
const PALETTE_SYMBOLS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+,-/:;<=>?@[]^_{}~";

#[derive(Resource, Default)]
struct StructureSelection {
    start: Option<IVec3>,
    slot: Option<usize>,
}

impl StructureSelection {
    fn clear(&mut self) {
        self.start = None;
        self.slot = None;
    }
}

#[derive(Serialize)]
struct ExportedStructure {
    id: String,
    name: ExportedStructureName,
    locatable: bool,
    anchor: ExportedAnchor,
    palette: BTreeMap<String, ExportedPaletteEntry>,
    layers: Vec<ExportedLayer>,
}

#[derive(Serialize)]
struct ExportedStructureName {
    english: &'static str,
    portuguese_brazil: &'static str,
    spanish: &'static str,
}

#[derive(Serialize)]
struct ExportedAnchor {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Serialize)]
struct ExportedPaletteEntry {
    block: String,
    orientation: &'static str,
}

#[derive(Serialize)]
struct ExportedLayer {
    y: i32,
    rows: Vec<String>,
}

struct ExportResult {
    path: PathBuf,
    occupied_blocks: usize,
    volume: usize,
}

pub(super) struct StructureToolPlugin;

impl Plugin for StructureToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StructureSelection>()
            .add_systems(
                OnEnter(GameState::Gameplay),
                reset_resource::<StructureSelection>,
            )
            .add_systems(
                Update,
                (
                    clear_selection_on_hotbar_change,
                    handle_structure_tool_use,
                )
                    .chain()
                    .after(BlockTargetingSet::Interaction)
                    .run_if(world_interaction_available),
            )
            .add_systems(
                Update,
                draw_structure_selection
                    .in_set(BlockTargetingSet::Visuals)
                    .run_if(world_interaction_available),
            );
    }
}

fn clear_selection_on_hotbar_change(
    hotbar: Res<PlayerHotbar>,
    mut selection: ResMut<StructureSelection>,
    mut chat: ResMut<ChatState>,
) {
    let Some(slot) = selection.slot else {
        return;
    };
    if hotbar.selected_slot() == slot
        && hotbar.item_at(slot) == Some(STRUCTURE_TOOL_ID)
    {
        return;
    }

    selection.clear();
    chat.append_text("selection cleared");
}

fn handle_structure_tool_use(
    mut uses: MessageReader<ToolUse>,
    hotbar: Res<PlayerHotbar>,
    camera: Single<&Transform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    mut selection: ResMut<StructureSelection>,
    mut chat: ResMut<ChatState>,
) {
    for usage in uses.read() {
        if usage.tool_id != STRUCTURE_TOOL_ID || usage.button != ToolUseButton::Right {
            continue;
        }
        let Some(hit) = usage.target else {
            continue;
        };
        let Some(point) = placement_voxel(hit, &world, camera.translation) else {
            continue;
        };

        let Some(start) = selection.start else {
            selection.start = Some(point);
            selection.slot = Some(hotbar.selected_slot());
            chat.append_text(format!(
                "selection started at X: {} Z: {} Y: {}",
                point.x, point.z, point.y
            ));
            continue;
        };

        selection.clear();
        chat.append_text(format!(
            "selection finished at X: {} Z: {} Y: {}",
            point.x, point.z, point.y
        ));

        match export_structure(&world, start, point) {
            Ok(result) => {
                chat.append_text(format!(
                    "{} blocks ready inside selection ({} positions).",
                    result.occupied_blocks, result.volume
                ));
                chat.append_structure_file(result.path);
            }
            Err(error) => {
                chat.append_text(format!("structure export failed: {error}"));
            }
        }
    }
}

fn draw_structure_selection(
    selection: Res<StructureSelection>,
    targeted: Res<TargetedBlock>,
    camera: Single<&Transform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    mut gizmos: Gizmos,
) {
    let Some(start) = selection.start else {
        return;
    };
    let end = targeted
        .0
        .and_then(|hit| placement_voxel(hit, &world, camera.translation))
        .unwrap_or(start);
    let (minimum, maximum) = selection_bounds(start, end);
    let size = (maximum - minimum + IVec3::ONE).as_vec3();
    let center = minimum.as_vec3() + size * 0.5;

    gizmos.cuboid(
        Transform::from_translation(center).with_scale(size),
        SELECTION_COLOR,
    );
}

fn selection_bounds(first: IVec3, second: IVec3) -> (IVec3, IVec3) {
    (first.min(second), first.max(second))
}

fn export_structure(
    world: &VoxelWorld,
    first: IVec3,
    second: IVec3,
) -> Result<ExportResult, String> {
    let (minimum, maximum) = selection_bounds(first, second);
    let width = usize::try_from(maximum.x - minimum.x + 1)
        .map_err(|_| "selection width is invalid".to_owned())?;
    let height = usize::try_from(maximum.y - minimum.y + 1)
        .map_err(|_| "selection height is invalid".to_owned())?;
    let depth = usize::try_from(maximum.z - minimum.z + 1)
        .map_err(|_| "selection depth is invalid".to_owned())?;
    let volume = width
        .checked_mul(height)
        .and_then(|value| value.checked_mul(depth))
        .ok_or_else(|| "selection volume is too large".to_owned())?;

    let mut palette_symbols = BTreeMap::<(String, u8), char>::new();
    let mut palette = BTreeMap::<String, ExportedPaletteEntry>::new();
    let mut layers = Vec::with_capacity(height);
    let mut occupied_blocks = 0_usize;

    for y in minimum.y..=maximum.y {
        let mut rows = Vec::with_capacity(depth);
        for z in minimum.z..=maximum.z {
            let mut row = String::with_capacity(width);
            for x in minimum.x..=maximum.x {
                let position = IVec3::new(x, y, z);
                if !world.is_loaded_at(position) {
                    return Err(format!(
                        "selection contains unloaded position X: {} Z: {} Y: {}",
                        position.x, position.z, position.y
                    ));
                }
                let Some(cell) = world.cell_at(position) else {
                    row.push('.');
                    continue;
                };

                occupied_blocks += 1;
                let key = (cell.block_id.to_owned(), cell.orientation.index());
                let symbol = match palette_symbols.get(&key).copied() {
                    Some(symbol) => symbol,
                    None => {
                        let symbol = palette_symbol(palette_symbols.len())?;
                        palette_symbols.insert(key, symbol);
                        palette.insert(
                            symbol.to_string(),
                            ExportedPaletteEntry {
                                block: cell.block_id.to_owned(),
                                orientation: orientation_name(cell.orientation),
                            },
                        );
                        symbol
                    }
                };
                row.push(symbol);
            }
            rows.push(row);
        }
        layers.push(ExportedLayer {
            y: y - minimum.y,
            rows,
        });
    }

    if occupied_blocks == 0 {
        return Err("selection contains no blocks".to_owned());
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f").to_string();
    let model = ExportedStructure {
        id: format!("asteria:structure_{timestamp}"),
        name: ExportedStructureName {
            english: "Temporary Structure",
            portuguese_brazil: "Estrutura Temporária",
            spanish: "Estructura Temporal",
        },
        locatable: false,
        anchor: ExportedAnchor { x: 0, y: 0, z: 0 },
        palette,
        layers,
    };
    let json = serde_json::to_string_pretty(&model)
        .map_err(|error| format!("failed to serialize structure: {error}"))?;

    let directory = std::env::temp_dir().join("structures");
    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "failed to create temporary structure directory {}: {error}",
            directory.display()
        )
    })?;
    let path = directory.join(format!("{timestamp}.json"));
    fs::write(&path, format!("{json}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

    Ok(ExportResult {
        path,
        occupied_blocks,
        volume,
    })
}

fn orientation_name(orientation: BlockOrientation) -> &'static str {
    match orientation {
        BlockOrientation::X => "x",
        BlockOrientation::Y => "y",
        BlockOrientation::Z => "z",
    }
}

fn palette_symbol(index: usize) -> Result<char, String> {
    if let Some(&symbol) = PALETTE_SYMBOLS.get(index) {
        return Ok(char::from(symbol));
    }

    let private_index = index - PALETTE_SYMBOLS.len();
    let codepoint = 0xE000_u32
        .checked_add(
            u32::try_from(private_index)
                .map_err(|_| "structure palette contains too many entries".to_owned())?,
        )
        .ok_or_else(|| "structure palette contains too many entries".to_owned())?;
    char::from_u32(codepoint)
        .ok_or_else(|| "structure palette contains too many entries".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_bounds_are_order_independent() {
        let a = IVec3::new(8, 2, -4);
        let b = IVec3::new(2, 7, -9);
        assert_eq!(
            selection_bounds(a, b),
            (IVec3::new(2, 2, -9), IVec3::new(8, 7, -4))
        );
    }

    #[test]
    fn palette_symbols_never_use_air_marker() {
        for index in 0..(PALETTE_SYMBOLS.len() + 32) {
            assert_ne!(palette_symbol(index).unwrap(), '.');
        }
    }
}
