use bevy::prelude::*;

use crate::{player::camera::GameplayCamera, rendering::environment::EnvironmentVisualState};

pub(super) fn update_fog_color(
    visuals: Res<EnvironmentVisualState>,
    mut fogs: Query<&mut DistanceFog, With<GameplayCamera>>,
) {
    if !visuals.is_changed() {
        return;
    }

    let color = visuals.fog_color.to_color();
    for mut fog in &mut fogs {
        if fog.color != color {
            fog.color = color;
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::color::Hsi;

    #[test]
    fn fog_uses_authored_fog_palette_independently_of_sky() {
        let mut app = App::new();
        let fog_color = Hsi::new(190.0, 0.6, 0.72);
        app.insert_resource(EnvironmentVisualState {
            sky_color: Hsi::new(248.0, 0.8, 0.13),
            fog_color,
            sky_light_factor: 1.0,
        });
        app.add_systems(Update, update_fog_color);
        let camera = app
            .world_mut()
            .spawn((GameplayCamera::default(), DistanceFog::default()))
            .id();

        app.update();

        assert_eq!(
            app.world().entity(camera).get::<DistanceFog>().unwrap().color,
            fog_color.to_color()
        );
    }
}
