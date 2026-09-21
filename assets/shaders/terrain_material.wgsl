#import bevy_pbr::{
    pbr_bindings,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    clustered_forward as clustering,
    forward_io::{VertexOutput, FragmentOutput},
    lighting,
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    pbr_functions::main_pass_post_lighting_processing,
    shadows,
    view_transformations,
}
#endif

struct TerrainMaterialExtension {
    fluid_animation_factor: f32,
    base_tint_enabled: f32,
    overlay_enabled: f32,
    overlay_tint_enabled: f32,
    texture_array_enabled: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<storage, read> terrain_global_lighting: array<vec4<f32>>;

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var<uniform> terrain_material_extension: TerrainMaterialExtension;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var terrain_overlay_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(103)
var terrain_overlay_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(104)
var terrain_texture_array: texture_2d_array<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(105)
var terrain_texture_array_sampler: sampler;

const AMBIENT_FLOOR: f32 = 0.055;
const SKY_LIGHT_GAMMA: f32 = 1.35;
const BLOCK_LIGHT_INTENSITY_GAMMA: f32 = 0.50;
const BLOCK_LIGHT_COLOR_STRENGTH: f32 = 0.72;
const SUN_AMBIENT_SHARE: f32 = 0.62;
const DYNAMIC_LIGHT_SCALE: f32 = 0.08;
const TINT_LUMINANCE_WEIGHTS: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

fn apply_layer_tint(source: vec3<f32>, tint: vec3<f32>) -> vec3<f32> {
    let source_luma = max(dot(source, TINT_LUMINANCE_WEIGHTS), 0.0);
    let tint_peak = max(max(tint.r, tint.g), max(tint.b, 0.001));
    let hue = tint / tint_peak;
    let hue_luma = max(dot(hue, TINT_LUMINANCE_WEIGHTS), 0.001);
    let luminance_compensation = min(2.0, 1.0 / hue_luma);

    return clamp(
        vec3<f32>(source_luma) * hue * luminance_compensation * 1.08,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
}

#ifndef PREPASS_PIPELINE
fn directional_sun_visibility(in: VertexOutput) -> f32 {
    let view_z = view_transformations::position_world_to_view(in.world_position.xyz).z;
    let surface_normal = normalize(in.world_normal);
    let directional_light_count = view_bindings::lights.n_directional_lights;

    for (var light_id: u32 = 0u; light_id < directional_light_count; light_id = light_id + 1u) {
        let light = &view_bindings::lights.directional_lights[light_id];
        let casts_shadows = ((*light).flags
            & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u;

        if !casts_shadows {
            continue;
        }

        let shadow = shadows::fetch_directional_shadow(
            light_id,
            in.world_position,
            surface_normal,
            view_z,
            in.position.xy,
        );
        let incidence = max(
            dot(surface_normal, normalize((*light).direction_to_light)),
            0.0,
        );

        return mix(SUN_AMBIENT_SHARE, 1.0, shadow * incidence);
    }

    return 1.0;
}

fn dynamic_point_lighting(
    in: VertexOutput,
    surface_normal: vec3<f32>,
    is_orthographic: bool,
) -> vec3<f32> {
    let view_z = dot(
        vec4<f32>(
            view_bindings::view.view_from_world[0].z,
            view_bindings::view.view_from_world[1].z,
            view_bindings::view.view_from_world[2].z,
            view_bindings::view.view_from_world[3].z,
        ),
        in.world_position,
    );
    let cluster_index = clustering::view_fragment_cluster_index(
        in.position.xy,
        view_z,
        is_orthographic,
    );
    let ranges = clustering::unpack_clusterable_object_index_ranges(cluster_index);
    var result = vec3<f32>(0.0);

    for (
        var index = ranges.first_point_light_index_offset;
        index < ranges.first_spot_light_index_offset;
        index = index + 1u
    ) {
        let light_id = clustering::get_clusterable_object_id(index);
        let light = &view_bindings::clustered_lights.data[light_id];
        let to_light = (*light).position_radius.xyz - in.world_position.xyz;
        let distance_squared = max(dot(to_light, to_light), 0.0001);
        let light_direction = to_light * inverseSqrt(distance_squared);
        let incidence = max(dot(surface_normal, light_direction), 0.0);

        if incidence <= 0.0 {
            continue;
        }

        let attenuation = lighting::getDistanceAttenuation(
            distance_squared,
            (*light).color_inverse_square_range.w,
        );
        var visibility = 1.0;
        let casts_shadows = ((*light).flags
            & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u;

        if casts_shadows {
            visibility = shadows::fetch_point_shadow(
                light_id,
                in.world_position,
                surface_normal,
                in.position.xy,
            );
        }

        result += (*light).color_inverse_square_range.rgb
            * attenuation
            * incidence
            * visibility
            * DYNAMIC_LIGHT_SCALE;
    }

    return result;
}
#endif

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Greedy terrain quads carry UVs larger than 1 so each merged voxel face
    // keeps the original per-block texture scale even when the sampler itself
    // is clamped.
    let tiled_uv = fract(in.uv);
    var texel = vec4<f32>(1.0);
    let packed_block_light = u32(round(in.color.x));
    let block_levels = vec3<f32>(
        f32(packed_block_light & 15u),
        f32((packed_block_light >> 4u) & 15u),
        f32((packed_block_light >> 8u) & 15u),
    ) / 15.0;

    let packed_tint = u32(round(in.color.y));
    let tint = vec3<f32>(
        f32(packed_tint & 255u),
        f32((packed_tint >> 8u) & 255u),
        f32((packed_tint >> 16u) & 255u),
    ) / 255.0;

    let packed_sky_material = u32(round(in.color.w));
    let material_code = packed_sky_material & 1048575u;
    let sky_level = f32((packed_sky_material >> 20u) & 15u) / 15.0;
    var base_tint_enabled = terrain_material_extension.base_tint_enabled > 0.5;
    var overlay_enabled = terrain_material_extension.overlay_enabled > 0.5;
    var overlay_tint_enabled = terrain_material_extension.overlay_tint_enabled > 0.5;
    let texture_array_enabled = terrain_material_extension.texture_array_enabled > 0.5;
    var array_overlay_index = 511u;

    if texture_array_enabled {
        let base_index = material_code & 511u;
        array_overlay_index = (material_code >> 9u) & 511u;
        let flags = (material_code >> 18u) & 3u;

        texel = textureSample(
            terrain_texture_array,
            terrain_texture_array_sampler,
            tiled_uv,
            i32(base_index),
        );
        base_tint_enabled = (flags & 1u) != 0u;
        overlay_enabled = array_overlay_index != 511u;
        overlay_tint_enabled = (flags & 2u) != 0u;
    } else {
        texel = textureSample(
            pbr_bindings::base_color_texture,
            pbr_bindings::base_color_sampler,
            tiled_uv,
        );
    }

    let fluid_animation = clamp(
        terrain_material_extension.fluid_animation_factor,
        0.0,
        1.0,
    );
    let is_fluid = fluid_animation > 0.5;
    let ambient_occlusion = clamp(in.color.z, 0.0, 1.0);
    let sky_light = pow(sky_level, SKY_LIGHT_GAMMA) * terrain_global_lighting[0].x;

    // Block-light channels are exact 4-bit voxel levels packed into COLOR.x.
    // Strength is shaped from the peak only, so boosting dim light never raises
    // the weaker color channels and therefore cannot wash red+blue toward white.
    let block_peak = max(max(block_levels.r, block_levels.g), block_levels.b);
    let block_hue = block_levels / max(block_peak, 0.001);
    let block_intensity = pow(block_peak, BLOCK_LIGHT_INTENSITY_GAMMA);

#ifdef PREPASS_PIPELINE
    let sun_visibility = 1.0;
    let dynamic_light = vec3<f32>(0.0);
#else
    var sun_visibility = 1.0;
    var dynamic_light = vec3<f32>(0.0);

    // Fluids are large blended surfaces and already receive propagated voxel
    // lighting. Sampling directional shadow maps and clustered point lights for
    // every transparent water fragment is disproportionately expensive and also
    // introduces unstable shadow artifacts on moving/translucent surfaces.
    if !is_fluid {
        let surface_normal = normalize(pbr_input.world_normal);
        sun_visibility = directional_sun_visibility(in);
        dynamic_light = dynamic_point_lighting(
            in,
            surface_normal,
            pbr_input.is_orthographic,
        );
    }
#endif

    let shadowed_sky_light = clamp(sky_light * sun_visibility, 0.0, 1.0);
    let sky_local_light = mix(AMBIENT_FLOOR, 1.0, shadowed_sky_light);

    // Sky light controls luminance, but it must not erase the hue of a strong
    // voxel light. Keep most of the propagated hue while leaving a small white
    // component so saturated colors do not make the scene unnaturally dark.
    let combined_intensity =
        1.0 - (1.0 - sky_local_light) * (1.0 - block_intensity);
    let block_hue_weight = smoothstep(0.08, 0.55, block_intensity);
    let color_weight = block_hue_weight * BLOCK_LIGHT_COLOR_STRENGTH;
    let combined_hue = mix(vec3<f32>(1.0), block_hue, color_weight);
    let local_light = combined_hue * combined_intensity * ambient_occlusion;

    var base_rgb = texel.rgb;
    if base_tint_enabled {
        base_rgb = apply_layer_tint(texel.rgb, tint);
    }

    if overlay_enabled {
        var overlay = vec4<f32>(1.0);
        if texture_array_enabled {
            overlay = textureSample(
                terrain_texture_array,
                terrain_texture_array_sampler,
                tiled_uv,
                i32(array_overlay_index),
            );
        } else {
            overlay = textureSample(
                terrain_overlay_texture,
                terrain_overlay_sampler,
                tiled_uv,
            );
        }
        var overlay_rgb = overlay.rgb;
        if overlay_tint_enabled {
            overlay_rgb = apply_layer_tint(overlay.rgb, tint);
        }
        base_rgb = mix(base_rgb, overlay_rgb, overlay.a);
    }

    var material_rgb = base_rgb * pbr_bindings::material.base_color.rgb;

#ifndef PREPASS_PIPELINE
    if is_fluid {
        let time = view_bindings::globals.time;
        let wave_a = sin(
            in.world_position.x * 0.34
                + in.world_position.z * 0.22
                + time * 1.35
        );
        let wave_b = sin(
            (in.world_position.x - in.world_position.z) * 0.72
                - time * 0.96
        );
        let moving_wave = wave_a * 0.68 + wave_b * 0.32;
        let ripple_ridge = smoothstep(
            0.52,
            0.94,
            abs(wave_a - wave_b) * 0.5,
        );
        let fluid_variation = moving_wave * 0.045;

        material_rgb = clamp(
            material_rgb * (1.0 + fluid_variation)
                + vec3<f32>(0.018, 0.028, 0.042) * ripple_ridge,
            vec3<f32>(0.0),
            vec3<f32>(1.0),
        );
    }
#endif

    // Dynamic held-item light is useful in darkness, but white point light must
    // not wash out a saturated voxel-light gradient that is already authoritative.
    let lighting_multiplier =
        local_light + dynamic_light * (1.0 - block_hue_weight);
    pbr_input.material.base_color = vec4<f32>(
        material_rgb * lighting_multiplier,
        texel.a * pbr_bindings::material.base_color.a,
    );
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = pbr_input.material.base_color;
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
