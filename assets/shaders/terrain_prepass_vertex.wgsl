#import bevy_pbr::{
    mesh_functions,
    prepass_io::VertexOutput,
    view_transformations::position_world_to_clip,
}

struct TerrainPrepassVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(1) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(2) payload: u32,
#endif
#ifdef VERTEX_COLORS
    @location(7) color: vec4<f32>,
#endif
}

fn voxel_local_normal(payload: u32) -> vec3<f32> {
    let code = (payload >> 21u) & 7u;
    switch code {
        case 0u: { return vec3<f32>(1.0, 0.0, 0.0); }
        case 1u: { return vec3<f32>(-1.0, 0.0, 0.0); }
        case 2u: { return vec3<f32>(0.0, 1.0, 0.0); }
        case 3u: { return vec3<f32>(0.0, -1.0, 0.0); }
        case 4u: { return vec3<f32>(0.0, 0.0, 1.0); }
        default: { return vec3<f32>(0.0, 0.0, -1.0); }
    }
}

fn decoded_uv_b(payload: u32) -> vec2<f32> {
    let sky_light = f32((payload >> 24u) & 15u) / 15.0;
    let tint_normal = payload & 0x00ffffffu;
    return vec2<f32>(sky_light, f32(tint_normal));
}

@vertex
fn vertex(vertex: TerrainPrepassVertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local =
        mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);

#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_UVS_B
    out.uv_b = decoded_uv_b(vertex.payload);
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_UVS_B
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        voxel_local_normal(vertex.payload),
        vertex.instance_index,
    );
#else
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
#endif
#endif

#ifdef MOTION_VECTOR_PREPASS
    let previous_world_from_local =
        mesh_functions::get_previous_world_from_local(vertex.instance_index);
    out.previous_world_position = mesh_functions::mesh_position_local_to_world(
        previous_world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither =
        mesh_functions::get_visibility_range_dither_level(
            vertex.instance_index,
            world_from_local[3],
        );
#endif

    return out;
}
