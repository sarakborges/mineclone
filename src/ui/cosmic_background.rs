use bevy::prelude::*;

pub const STAR_FIELD: &[(f32, f32, f32, f32, f32, f32, f32, f32, f32)] = &[
    (7.0, 12.0, 2.4, 0.1, 0.38, 0.88, 0.91, 1.0, 0.62),
    (15.0, 73.0, 2.8, 1.7, 0.31, 0.72, 0.61, 1.0, 0.54),
    (22.0, 28.0, 2.0, 2.4, 0.44, 0.67, 0.89, 1.0, 0.58),
    (29.0, 88.0, 2.2, 0.7, 0.35, 0.96, 0.92, 1.0, 0.50),
    (34.0, 16.0, 1.8, 3.2, 0.41, 0.72, 0.64, 1.0, 0.47),
    (41.0, 68.0, 2.7, 4.1, 0.30, 0.90, 0.95, 1.0, 0.60),
    (47.0, 9.0, 2.0, 1.1, 0.43, 0.64, 0.86, 1.0, 0.51),
    (53.0, 82.0, 2.4, 2.9, 0.33, 0.82, 0.66, 1.0, 0.53),
    (59.0, 21.0, 1.9, 0.5, 0.39, 0.97, 0.96, 1.0, 0.56),
    (66.0, 61.0, 3.0, 3.8, 0.28, 0.58, 0.82, 1.0, 0.64),
    (72.0, 34.0, 2.1, 5.0, 0.36, 0.76, 0.61, 1.0, 0.52),
    (79.0, 84.0, 1.9, 1.9, 0.42, 0.91, 0.94, 1.0, 0.55),
    (85.0, 18.0, 2.8, 4.6, 0.32, 0.62, 0.86, 1.0, 0.61),
    (91.0, 69.0, 2.2, 2.2, 0.40, 0.83, 0.64, 1.0, 0.53),
    (95.0, 39.0, 1.8, 3.4, 0.34, 0.92, 0.95, 1.0, 0.48),
    (11.0, 47.0, 1.9, 5.4, 0.29, 0.59, 0.83, 1.0, 0.49),
    (25.0, 55.0, 2.3, 1.3, 0.37, 0.93, 0.94, 1.0, 0.51),
    (76.0, 11.0, 2.0, 2.7, 0.34, 0.74, 0.61, 1.0, 0.50),
];

#[derive(Component)]
pub struct CosmicBackgroundStar {
    base_left: f32,
    base_top: f32,
    base_size: f32,
    phase: f32,
    speed: f32,
    red: f32,
    green: f32,
    blue: f32,
    base_alpha: f32,
}

pub fn star(
    left: f32,
    top: f32,
    size: f32,
    phase: f32,
    speed: f32,
    red: f32,
    green: f32,
    blue: f32,
    base_alpha: f32,
) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: percent(left),
            top: percent(top),
            width: px(size),
            height: px(size),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(Color::srgba(red, green, blue, base_alpha)),
        BoxShadow(vec![ShadowStyle {
            color: Color::srgba(red, green, blue, base_alpha * 0.55),
            x_offset: px(0),
            y_offset: px(0),
            spread_radius: px(0.5),
            blur_radius: px(7.0),
        }]),
        CosmicBackgroundStar {
            base_left: left,
            base_top: top,
            base_size: size,
            phase,
            speed,
            red,
            green,
            blue,
            base_alpha,
        },
        Pickable::IGNORE,
    )
}

pub fn animate_stars(
    time: Res<Time<Real>>,
    mut stars: Query<(
        &CosmicBackgroundStar,
        &mut BackgroundColor,
        &mut BoxShadow,
        &mut Node,
    )>,
) {
    let elapsed = time.elapsed_secs();

    for (star, mut background, mut shadow, mut node) in &mut stars {
        let wave = (elapsed * star.speed * 2.35 + star.phase).sin() * 0.5 + 0.5;
        let shimmer = 0.34 + 0.66 * wave.powf(1.35);
        let alpha = star.base_alpha * shimmer;
        let pulse = 0.88 + 0.24 * wave;
        let drift_time = elapsed * (0.07 + star.speed * 0.07);
        let drift_x = (drift_time + star.phase).sin() * 1.15;
        let drift_y = (drift_time * 0.72 + star.phase * 1.37).cos() * 0.78;

        node.left = percent(star.base_left + drift_x);
        node.top = percent(star.base_top + drift_y);
        node.width = px(star.base_size * pulse);
        node.height = px(star.base_size * pulse);

        *background = Color::srgba(star.red, star.green, star.blue, alpha).into();
        *shadow = BoxShadow(vec![ShadowStyle {
            color: Color::srgba(star.red, star.green, star.blue, alpha * 0.62),
            x_offset: px(0),
            y_offset: px(0),
            spread_radius: px(0.5),
            blur_radius: px(6.0 + 3.0 * wave),
        }]);
    }
}
