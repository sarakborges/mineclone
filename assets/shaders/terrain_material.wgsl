#import bevy_pbr::{
    pbr_bindings,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings as view_bindings,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct TerrainMaterialExtension {
    sky_light_factor: f32,
    fluid_animation_factor: f32,
    _padding: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> terrain_material_extension: TerrainMaterialExtension;

const PACKED_RGB_MAX: f32 = 16777215.0;
const LIGHT_RESPONSE_GAMMA: f32 = 1.35;
const AMBIENT_FLOOR: f32 = 0.015;
const SKY_AMBIENT_STRENGTH: f32 = 0.24;
const BLOCK_LIGHT_STRENGTH: f32 = 0.85;

fn unpack_rgb(value: f32) -> vec3<f32> {
    let packed = round(clamp(value, 0.0, 1.0) * PACKED_RGB_MAX);
    let red = floor(packed / 65536.0);
    let remainder = packed - red * 65536.0;
    let green = floor(remainder / 256.0);
    let blue = remainder - green * 256.0;

    return vec3<f32>(red, green, blue) / 255.0;
}

fn srgb_channel_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        return value / 12.92;
    }

    return pow((value + 0.055) / 1.055, 2.4);
}

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_channel_to_linear(color.r),
        srgb_channel_to_linear(color.g),
        srgb_channel_to_linear(color.b),
    );
}

fn apply_authored_tint(texel: vec4<f32>, tint_srgb: vec3<f32>) -> vec3<f32> {
    let tint_delta = max(
        abs(1.0 - tint_srgb.r),
        max(abs(1.0 - tint_srgb.g), abs(1.0 - tint_srgb.b)),
    );

    if tint_delta <= 0.001 {
        return texel.rgb;
    }

    let tint = srgb_to_linear(tint_srgb);
    let luminance_weights = vec3<f32>(0.2126, 0.7152, 0.0722);
    let luminance = dot(texel.rgb, luminance_weights);
    let texture_max = max(max(texel.r, texel.g), texel.b);
    let texture_min = min(min(texel.r, texel.g), texel.b);
    let texture_chroma = texture_max - texture_min;

    // Biome/dye tinting is authored around desaturated masks. Transparent
    // texels are also tintable so colored glass and similar materials keep
    // their intended hue without flattening opaque colored details.
    let neutral_mask = 1.0 - smoothstep(0.025, 0.10, texture_chroma);
    let translucent_mask = 1.0 - smoothstep(0.985, 0.999, texel.a);
    let tint_mask = max(neutral_mask, translucent_mask);
    let shade = mix(0.55, 1.15, clamp(luminance, 0.0, 1.0));
    let tinted = clamp(tint * shade, vec3<f32>(0.0), vec3<f32>(1.0));

    return mix(texel.rgb, tinted, tint_mask);
}

fn light_response(light: vec3<f32>) -> vec3<f32> {
    return pow(
        clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)),
        vec3<f32>(LIGHT_RESPONSE_GAMMA),
    );
}

fn voxel_emissive(
    base_color: vec3<f32>,
    sky_light: vec3<f32>,
    block_light: vec3<f32>,
    ambient_occlusion: f32,
) -> vec3<f32> {
    let sky_fill = light_response(sky_light)
        * clamp(terrain_material_extension.sky_light_factor, 0.0, 1.0)
        * SKY_AMBIENT_STRENGTH;
    let block_fill = light_response(block_light) * BLOCK_LIGHT_STRENGTH;
    let local_fill = max(sky_fill, block_fill) + vec3<f32>(AMBIENT_FLOOR);

    return base_color * local_fill * ambient_occlusion;
}

#ifndef PREPASS_PIPELINE
fn animate_fluid_color(color: vec3<f32>, world_position: vec3<f32>) -> vec3<f32> {
    let amount = clamp(
        terrain_material_extension.fluid_animation_factor,
        0.0,
        1.0,
    );

    if amount <= 0.0 {
        return color;
    }

    let time = view_bindings::globals.time;
    let broad_wave = sin(
        world_position.x * 0.30
            + world_position.z * 0.22
            + time * 1.15
    );
    let cross_wave = cos(
        world_position.z * 0.37
            - world_position.x * 0.16
            + time * 0.87
    );
    let fine_wave = sin(
        (world_position.x + world_position.z) * 1.08
            + time * 1.43
    );
    let variation = (
        broad_wave * 0.022
            + cross_wave * 0.016
            + fine_wave * 0.008
    ) * amount;

    return clamp(
        color * (1.0 + variation),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
}
#endif

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Start from Bevy's StandardMaterial input so roughness, metallic,
    // normals, material flags and the forward/deferred paths stay native.
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Voxel vertex color and UV-B are intentionally repurposed for lighting
    // and tint data, so rebuild the albedo instead of using the vertex-color-
    // multiplied base color produced by StandardMaterial.
    let texel = textureSample(
        pbr_bindings::base_color_texture,
        pbr_bindings::base_color_sampler,
        in.uv,
    );
    let tint_srgb = unpack_rgb(in.uv_b.y);
    let material_base_color = pbr_bindings::material.base_color;
    var material_rgb = apply_authored_tint(texel, tint_srgb)
        * material_base_color.rgb;

#ifndef PREPASS_PIPELINE
    material_rgb = animate_fluid_color(material_rgb, in.world_position.xyz);
#endif

    pbr_input.material.base_color = vec4<f32>(
        material_rgb,
        texel.a * material_base_color.a,
    );
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

    let is_unlit = (
        pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT
    ) != 0u;

    if !is_unlit {
        let sky_light = unpack_rgb(in.uv_b.x);
        let block_light = clamp(
            in.color.rgb,
            vec3<f32>(0.0),
            vec3<f32>(1.0),
        );
        let ambient_occlusion = clamp(in.color.a, 0.0, 1.0);
        let baked_emissive = voxel_emissive(
            material_rgb,
            sky_light,
            block_light,
            ambient_occlusion,
        );

        // Voxel light is indirect/local fill. Bevy's PBR lighting remains
        // responsible for the directional sun, native point lights and their
        // shadows, avoiding the previous double/manual light implementation.
        pbr_input.material.emissive = vec4<f32>(
            pbr_input.material.emissive.rgb + baked_emissive,
            pbr_input.material.emissive.a,
        );
        pbr_input.diffuse_occlusion = pbr_input.diffuse_occlusion
            * vec3<f32>(ambient_occlusion);
        pbr_input.specular_occlusion = pbr_input.specular_occlusion
            * ambient_occlusion;
    }

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    if is_unlit {
        out.color = pbr_input.material.base_color;
    } else {
        out.color = apply_pbr_lighting(pbr_input);
    }

    // Keep Bevy's native fog, alpha handling, tonemapping and debanding in
    // the same order as the stock PBR shader.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
