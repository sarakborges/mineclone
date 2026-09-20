use bevy::prelude::*;

use super::chunk::CHUNK_SIZE;

pub(crate) fn chunk_coord_from_position(position: Vec3) -> IVec3 {
    let chunk_size = CHUNK_SIZE as f32;
    IVec3::new(
        (position.x / chunk_size).floor() as i32,
        (position.y / chunk_size).floor() as i32,
        (position.z / chunk_size).floor() as i32,
    )
}

pub(crate) fn chunk_coord_from_world(world_position: IVec3) -> IVec3 {
    let chunk_size = CHUNK_SIZE as i32;
    IVec3::new(
        world_position.x.div_euclid(chunk_size),
        world_position.y.div_euclid(chunk_size),
        world_position.z.div_euclid(chunk_size),
    )
}

pub(crate) fn split_world_position(world_position: IVec3) -> (IVec3, IVec3) {
    let chunk_size = CHUNK_SIZE as i32;
    let chunk = chunk_coord_from_world(world_position);
    let local = IVec3::new(
        world_position.x.rem_euclid(chunk_size),
        world_position.y.rem_euclid(chunk_size),
        world_position.z.rem_euclid(chunk_size),
    );

    (chunk, local)
}

pub(crate) fn chunk_origin(coord: IVec3) -> IVec3 {
    coord * CHUNK_SIZE as i32
}

pub(crate) fn chunks_for_block_extent(extent: i32) -> i32 {
    if extent <= 0 {
        return 0;
    }

    let chunk_size = CHUNK_SIZE as i32;
    extent.saturating_add(chunk_size - 1) / chunk_size
}

pub(crate) fn visit_chunk_coords_whose_voxel_halo_contains(
    world_position: IVec3,
    mut visit: impl FnMut(IVec3),
) {
    let (center, local) = split_world_position(world_position);
    let (x_offsets, x_len) = halo_axis_offsets(local.x);
    let (y_offsets, y_len) = halo_axis_offsets(local.y);
    let (z_offsets, z_len) = halo_axis_offsets(local.z);

    for &y in &y_offsets[..y_len] {
        for &z in &z_offsets[..z_len] {
            for &x in &x_offsets[..x_len] {
                let coord = center + IVec3::new(x, y, z);
                if coord.y >= 0 {
                    visit(coord);
                }
            }
        }
    }
}

fn halo_axis_offsets(local: i32) -> ([i32; 2], usize) {
    let last = CHUNK_SIZE as i32 - 1;
    if local == 0 {
        ([-1, 0], 2)
    } else if local == last {
        ([0, 1], 2)
    } else {
        ([0, 0], 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_world_position_splits_with_euclidean_coordinates() {
        let world = IVec3::new(-1, 17, -17);
        let (chunk, local) = split_world_position(world);

        assert_eq!(chunk, IVec3::new(-1, 1, -2));
        assert_eq!(local, IVec3::new(15, 1, 15));
        assert_eq!(chunk_origin(chunk) + local, world);
    }

    #[test]
    fn floating_position_maps_to_containing_chunk() {
        assert_eq!(
            chunk_coord_from_position(Vec3::new(-0.1, 16.0, -16.1)),
            IVec3::new(-1, 1, -2),
        );
    }

    #[test]
    fn voxel_halo_visits_only_chunks_whose_one_voxel_shell_contains_position() {
        let interior = IVec3::new(4, 20, 6);
        let mut interior_chunks = Vec::new();
        visit_chunk_coords_whose_voxel_halo_contains(interior, |coord| {
            interior_chunks.push(coord)
        });
        assert_eq!(interior_chunks, vec![IVec3::new(0, 1, 0)]);

        let corner = IVec3::new(15, 31, 15);
        let mut corner_chunks = Vec::new();
        visit_chunk_coords_whose_voxel_halo_contains(corner, |coord| {
            corner_chunks.push(coord)
        });
        corner_chunks.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));

        let mut expected = Vec::new();
        for y in [1, 2] {
            for z in [0, 1] {
                for x in [0, 1] {
                    expected.push(IVec3::new(x, y, z));
                }
            }
        }
        expected.sort_unstable_by_key(|coord| (coord.y, coord.z, coord.x));
        assert_eq!(corner_chunks, expected);
    }

    #[test]
    fn block_extent_rounds_up_to_whole_chunks() {
        assert_eq!(chunks_for_block_extent(0), 0);
        assert_eq!(chunks_for_block_extent(1), 1);
        assert_eq!(chunks_for_block_extent(CHUNK_SIZE as i32), 1);
        assert_eq!(chunks_for_block_extent(CHUNK_SIZE as i32 + 1), 2);
    }
}
