#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings as view_bindings,
    forward_io::VertexOutput,
    view_transformations::position_world_to_clip,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<storage, read> object_global_lighting: array<vec4<f32>>;

struct ObjectPrimitiveMaterialExtension {
    wind_sway: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var<uniform> object_primitive_material: ObjectPrimitiveMaterialExtension;

struct ObjectVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
}

fn apply_wind_sway(world_position: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    if object_primitive_material.wind_sway <= 0.5 {
        return world_position;
    }

    let wind_velocity = object_global_lighting[0].zw;
    let wind_speed = length(wind_velocity);
    if wind_speed <= 0.0001 {
        return world_position;
    }

    let influence = pow(clamp(1.0 - uv.y, 0.0, 1.0), 1.65);
    if influence <= 0.0001 {
        return world_position;
    }

    let wind_direction = wind_velocity / wind_speed;
    let perpendicular = vec2<f32>(-wind_direction.y, wind_direction.x);
    let phase = world_position.x * 0.23
        + world_position.z * 0.19
        + view_bindings::globals.time * (1.55 + wind_speed * 0.85);
    let primary = sin(phase);
    let secondary = sin(phase * 0.57 + 1.91);
    let amplitude = min(wind_speed * 0.16, 0.105) * influence;
    let offset = wind_direction * (primary * amplitude)
        + perpendicular * (secondary * amplitude * 0.3);

    return world_position + vec4<f32>(offset.x, 0.0, offset.y, 0.0);
}

@vertex
fn vertex(vertex: ObjectVertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );

#ifdef VERTEX_UVS_A
    world_position = apply_wind_sway(world_position, vertex.uv);
    out.uv = vertex.uv;
#endif

    out.world_position = world_position;
    out.position = position_world_to_clip(world_position.xyz);

#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index,
    );
#else
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex.instance_index,
        world_from_local[3],
    );
#endif

    return out;
}
