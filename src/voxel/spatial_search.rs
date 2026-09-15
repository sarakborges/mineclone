use bevy::prelude::*;

pub(crate) fn find_map_square_rings<T>(
    center: IVec2,
    max_radius: i32,
    step: i32,
    mut resolve: impl FnMut(IVec2) -> Option<T>,
) -> Option<T> {
    assert!(max_radius >= 0, "square-ring radius must be nonnegative");
    assert!(step > 0, "square-ring step must be positive");

    for radius in 0..=max_radius {
        for z_offset in -radius..=radius {
            for x_offset in -radius..=radius {
                if radius > 0 && x_offset.abs() != radius && z_offset.abs() != radius {
                    continue;
                }

                let offset = IVec2::new(x_offset * step, z_offset * step);
                if let Some(result) = resolve(center + offset) {
                    return Some(result);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_ring_search_checks_center_first() {
        let center = IVec2::new(4, 7);
        let found = find_map_square_rings(center, 3, 2, |candidate| {
            (candidate == center).then_some(candidate)
        });

        assert_eq!(found, Some(center));
    }

    #[test]
    fn square_ring_search_scales_offsets_by_step() {
        let center = IVec2::new(10, 10);
        let target = IVec2::new(14, 6);
        let found = find_map_square_rings(center, 2, 2, |candidate| {
            (candidate == target).then_some(candidate)
        });

        assert_eq!(found, Some(target));
    }
}
