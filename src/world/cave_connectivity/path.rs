use bevy::prelude::*;

const TARGET_SEGMENT_LENGTH: f32 = 10.0;
const MIN_SEGMENTS: usize = 5;
const MAX_SEGMENTS: usize = 28;
const MAX_SIDE_AMPLITUDE: f32 = 22.0;
const MAX_VERTICAL_AMPLITUDE: f32 = 14.0;
const MIN_WORLD_Y: f32 = 2.5;
const ENDPOINT_CHAMBER_FRACTION: f32 = 0.16;
const ENDPOINT_CHAMBER_SCALE: f32 = 1.65;

pub(super) fn chaotic_connector_points(from: Vec3, to: Vec3, hash: u64) -> Vec<Vec3> {
    let delta = to - from;
    let distance = delta.length();

    if distance <= f32::EPSILON {
        return vec![from];
    }

    let direction = delta / distance;
    let reference = if direction.dot(Vec3::Y).abs() < 0.92 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let side = direction.cross(reference).normalize_or_zero();
    let up = side.cross(direction).normalize_or_zero();
    let segment_count =
        ((distance / TARGET_SEGMENT_LENGTH).ceil() as usize).clamp(MIN_SEGMENTS, MAX_SEGMENTS);

    let phase = hash_unit(hash.rotate_left(7)) * std::f32::consts::TAU;
    let secondary_phase = hash_unit(hash.rotate_left(19)) * std::f32::consts::TAU;
    let turns = lerp(0.8, 3.2, hash_unit(hash.rotate_left(31)));
    let side_amplitude = (distance * lerp(0.06, 0.14, hash_unit(hash.rotate_left(41))))
        .clamp(3.0, MAX_SIDE_AMPLITUDE);
    let vertical_amplitude = (distance * lerp(0.035, 0.09, hash_unit(hash.rotate_left(53))))
        .clamp(2.0, MAX_VERTICAL_AMPLITUDE);
    let broad_frequency = lerp(1.2, 2.8, hash_unit(hash.rotate_left(11)));
    let fine_frequency = lerp(3.0, 6.5, hash_unit(hash.rotate_left(23)));

    (0..=segment_count)
        .map(|index| {
            if index == 0 {
                return from;
            }
            if index == segment_count {
                return to;
            }

            let t = index as f32 / segment_count as f32;
            let envelope = (std::f32::consts::PI * t).sin().powf(1.15);
            let center = from.lerp(to, t);
            let helix_angle = phase + t * std::f32::consts::TAU * turns;
            let helix_side = helix_angle.cos() * side_amplitude * 0.55;
            let helix_up = helix_angle.sin() * vertical_amplitude * 0.55;
            let broad_side = (secondary_phase + t * std::f32::consts::TAU * broad_frequency).sin()
                * side_amplitude
                * 0.45;
            let broad_up = (phase * 0.7 + t * std::f32::consts::TAU * (broad_frequency * 0.73))
                .cos()
                * vertical_amplitude
                * 0.42;
            let fine_side = (phase * 1.9 + t * std::f32::consts::TAU * fine_frequency).sin()
                * side_amplitude
                * 0.18;
            let fine_up =
                (secondary_phase * 1.3 + t * std::f32::consts::TAU * (fine_frequency * 1.17)).sin()
                    * vertical_amplitude
                    * 0.16;
            let offset =
                side * (helix_side + broad_side + fine_side) + up * (helix_up + broad_up + fine_up);
            let mut point = center + offset * envelope;
            point.y = point.y.max(MIN_WORLD_Y);
            point
        })
        .collect()
}

pub(super) fn connector_radius_progress(start: f32, end: f32, t: f32, hash: u64) -> f32 {
    let base = start + (end - start) * t;
    let phase = hash_unit(hash.rotate_left(13)) * std::f32::consts::TAU;
    let swell = 1.0
        + (phase + t * std::f32::consts::TAU * 2.3).sin() * 0.12
        + (phase * 0.5 + t * std::f32::consts::TAU * 5.1).sin() * 0.05;
    let width_floor = start.max(end) * 0.90;
    let chamber = endpoint_chamber_scale(t);

    (base * swell * chamber).max(width_floor).max(2.5)
}

fn endpoint_chamber_scale(t: f32) -> f32 {
    let edge_distance = t.clamp(0.0, 1.0).min(1.0 - t.clamp(0.0, 1.0));
    if edge_distance >= ENDPOINT_CHAMBER_FRACTION {
        return 1.0;
    }

    let influence = 1.0 - edge_distance / ENDPOINT_CHAMBER_FRACTION;
    lerp(1.0, ENDPOINT_CHAMBER_SCALE, smoothstep(influence))
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_path_preserves_exact_endpoints() {
        let from = Vec3::new(10.0, 30.0, 20.0);
        let to = Vec3::new(120.0, 50.0, -15.0);
        let points = chaotic_connector_points(from, to, 42);

        assert_eq!(points.first().copied(), Some(from));
        assert_eq!(points.last().copied(), Some(to));
    }

    #[test]
    fn connector_path_is_deterministic() {
        let from = Vec3::new(10.0, 30.0, 20.0);
        let to = Vec3::new(120.0, 50.0, -15.0);

        assert_eq!(
            chaotic_connector_points(from, to, 99),
            chaotic_connector_points(from, to, 99)
        );
    }

    #[test]
    fn connector_path_is_not_a_straight_segment() {
        let from = Vec3::new(0.0, 30.0, 0.0);
        let to = Vec3::new(120.0, 30.0, 0.0);
        let points = chaotic_connector_points(from, to, 1234);

        assert!(
            points[1..points.len() - 1]
                .iter()
                .any(|point| point.y != 30.0 || point.z != 0.0)
        );
    }

    #[test]
    fn connector_width_does_not_collapse_between_anchors() {
        let start = 7.0;
        let end = 4.0;
        let minimum = start.max(end) * 0.90;

        for step in 0..=20 {
            let t = step as f32 / 20.0;
            assert!(connector_radius_progress(start, end, t, 42) >= minimum);
        }
    }

    #[test]
    fn connector_endpoints_open_into_wider_chambers() {
        let start = connector_radius_progress(5.0, 5.0, 0.0, 42);
        let middle = connector_radius_progress(5.0, 5.0, 0.5, 42);
        let end = connector_radius_progress(5.0, 5.0, 1.0, 42);

        assert!(start > middle);
        assert!(end > middle);
    }
}
