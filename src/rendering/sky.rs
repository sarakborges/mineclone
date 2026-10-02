use bevy::prelude::*;

use crate::app::game_state::GameState;

use super::environment::EnvironmentVisualState;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_sky_color
                .run_if(in_state(GameState::Gameplay))
                .run_if(sky_visuals_changed),
        );
    }
}

fn sky_visuals_changed(visuals: Res<EnvironmentVisualState>) -> bool {
    visuals.is_changed()
}

fn update_sky_color(visuals: Res<EnvironmentVisualState>, mut clear_color: ResMut<ClearColor>) {
    if !visuals.is_changed() {
        return;
    }

    // The flat background sits behind every world fragment. DistanceFog at
    // full opacity replaces a fragment with the authored fog color, so the
    // background must use that same terminal color or fully fogged geometry
    // remains visible as a silhouette against the sky. A future sky-gradient
    // renderer can preserve the authored sky palette away from the horizon
    // while still converging to fog_color where world geometry disappears.
    let color = visuals.fog_color.to_color();
    if clear_color.0 != color {
        clear_color.0 = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::color::Hsi;

    #[test]
    fn clear_color_matches_terminal_fog_color() {
        let mut app = App::new();
        app.insert_resource(EnvironmentVisualState::default());
        app.insert_resource(ClearColor(Color::BLACK));
        app.add_systems(Update, update_sky_color);

        app.update();
        let authored_sky = Hsi::new(275.0, 0.9, 0.45);
        let authored_fog = Hsi::new(45.0, 1.0, 0.9);
        {
            let mut visuals = app.world_mut().resource_mut::<EnvironmentVisualState>();
            visuals.sky_color = authored_sky;
            visuals.fog_color = authored_fog;
        }
        app.update();
        assert_eq!(app.world().resource::<ClearColor>().0, authored_fog.to_color());

        let next_fog = Hsi::new(180.0, 1.0, 0.2);
        app.world_mut()
            .resource_mut::<EnvironmentVisualState>()
            .fog_color = next_fog;
        app.update();
        assert_eq!(app.world().resource::<ClearColor>().0, next_fog.to_color());
    }
}
