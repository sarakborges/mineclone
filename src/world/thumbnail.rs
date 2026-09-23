use std::{
    fs, io,
    path::{Path, PathBuf},
};

use bevy::{
    camera::CameraOutputMode,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};

use crate::{
    app::pause_state::PauseState,
    player::camera::GameplayWorldCamera,
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
};

use super::world_names::{validate_world_name, WORLDS_DIRECTORY};

const THUMBNAIL_FILE_NAME: &str = "thumbnail.png";
const THUMBNAIL_WIDTH: u32 = 320;
const THUMBNAIL_HEIGHT: u32 = 180;

#[derive(Clone, Copy)]
pub(crate) enum WorldThumbnailCompletion {
    LeaveWorld,
    ExitGame,
}

#[derive(Component)]
pub(crate) struct WorldThumbnailCapture {
    world_id: String,
    completion: WorldThumbnailCompletion,
    camera_states: Vec<(Entity, CameraOutputMode, bool)>,
    delay_frames: u8,
}

pub(crate) fn begin_world_thumbnail_capture(
    commands: &mut Commands,
    cameras: &mut Query<(Entity, &mut Camera, Option<&GameplayWorldCamera>)>,
    world_id: &str,
    completion: WorldThumbnailCompletion,
) {
    let mut camera_states = Vec::new();
    let mut world_camera_found = false;
    for (entity, mut camera, world_camera) in cameras.iter_mut() {
        camera_states.push((entity, camera.output_mode, camera.is_active));
        if world_camera.is_some() {
            world_camera_found = true;
            camera.is_active = true;
            camera.output_mode = CameraOutputMode::Write {
                blend_state: None,
                clear_color: ClearColorConfig::Default,
            };
        } else {
            camera.is_active = false;
            camera.output_mode = CameraOutputMode::Skip;
        }
    }
    if !world_camera_found {
        warn!("world thumbnail capture started without a GameplayWorldCamera");
    }

    commands
        .spawn(WorldThumbnailCapture {
            world_id: world_id.to_owned(),
            completion,
            camera_states,
            delay_frames: 1,
        })
        .observe(finish_world_thumbnail_capture);
}

pub(crate) fn advance_world_thumbnail_capture(
    mut commands: Commands,
    mut captures: Query<(Entity, &mut WorldThumbnailCapture), Without<Screenshot>>,
) {
    for (entity, mut capture) in &mut captures {
        if capture.delay_frames > 0 {
            capture.delay_frames -= 1;
            continue;
        }

        commands.entity(entity).insert(Screenshot::primary_window());
    }
}

fn finish_world_thumbnail_capture(
    captured: On<ScreenshotCaptured>,
    captures: Query<&WorldThumbnailCapture>,
    mut cameras: Query<&mut Camera>,
    mut transition: ResMut<ScreenTransition>,
    mut app_exit: MessageWriter<AppExit>,
) {
    let Ok(capture) = captures.get(captured.entity) else {
        return;
    };

    for (entity, output_mode, is_active) in &capture.camera_states {
        if let Ok(mut camera) = cameras.get_mut(*entity) {
            camera.output_mode = *output_mode;
            camera.is_active = *is_active;
        }
    }

    if let Err(error) = write_world_thumbnail(&capture.world_id, &captured.image) {
        warn!(
            "World {} saved, but its thumbnail could not be written: {error}",
            capture.world_id
        );
    }

    match capture.completion {
        WorldThumbnailCompletion::LeaveWorld => {
            transition.request(
                ScreenTransitionTarget::game(crate::app::game_state::GameState::StartingScreen)
                    .with_pause(PauseState::Running),
            );
        }
        WorldThumbnailCompletion::ExitGame => {
            app_exit.write(AppExit::Success);
        }
    }
}

pub(crate) fn load_world_thumbnail(world_id: &str) -> io::Result<Option<Image>> {
    let path = world_thumbnail_path(world_id)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world thumbnail must be a regular file",
        ));
    }

    let dynamic = ::image::open(&path).map_err(io::Error::other)?;
    Ok(Some(Image::from_dynamic(dynamic, true, default())))
}

fn write_world_thumbnail(world_id: &str, image: &Image) -> io::Result<()> {
    let dynamic = image
        .clone()
        .try_into_dynamic()
        .map_err(io::Error::other)?;
    let thumbnail = dynamic.resize_to_fill(
        THUMBNAIL_WIDTH,
        THUMBNAIL_HEIGHT,
        ::image::imageops::FilterType::Lanczos3,
    );
    thumbnail
        .save_with_format(world_thumbnail_path(world_id)?, ::image::ImageFormat::Png)
        .map_err(io::Error::other)
}

fn world_thumbnail_path(world_id: &str) -> io::Result<PathBuf> {
    validate_world_name(world_id)?;
    Ok(Path::new(WORLDS_DIRECTORY)
        .join(world_id)
        .join(THUMBNAIL_FILE_NAME))
}


pub(crate) fn enforce_world_thumbnail_camera_isolation(
    captures: Query<(), With<WorldThumbnailCapture>>,
    mut cameras: Query<(&mut Camera, Option<&GameplayWorldCamera>)>,
) {
    if captures.is_empty() {
        return;
    }

    for (mut camera, world_camera) in &mut cameras {
        if world_camera.is_some() {
            camera.is_active = true;
            camera.output_mode = CameraOutputMode::Write {
                blend_state: None,
                clear_color: ClearColorConfig::Default,
            };
        } else {
            camera.is_active = false;
            camera.output_mode = CameraOutputMode::Skip;
        }
    }
}
