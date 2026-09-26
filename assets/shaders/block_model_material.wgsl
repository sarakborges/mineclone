#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
#endif

struct BlockModelMaterialExtension {
    tint: vec4<f32>,
    tint_enabled: f32,
    opacity: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> block_model_material: BlockModelMaterialExtension;

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

fn block_opacity_threshold(position: vec2<f32>) -> f32 {
    let thresholds = array<f32, 16>(
        0.0, 8.0, 2.0, 10.0,
        12.0, 4.0, 14.0, 6.0,
        3.0, 11.0, 1.0, 9.0,
        15.0, 7.0, 13.0, 5.0,
    );
    let x = u32(floor(position.x)) & 3u;
    let y = u32(floor(position.y)) & 3u;
    return (thresholds[y * 4u + x] + 0.5) / 16.0;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let base_color = pbr_input.material.base_color;

    if block_model_material.tint_enabled > 0.5 {
        let tint = clamp(block_model_material.tint.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        pbr_input.material.base_color = vec4<f32>(
            apply_layer_tint(base_color.rgb, tint),
            base_color.a,
        );
    }

    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

    let opacity = clamp(block_model_material.opacity, 0.0, 1.0);
    if opacity < 0.999 && opacity <= block_opacity_threshold(in.position.xy) {
        discard;
    }

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = pbr_input.material.base_color;
    return out;
#endif
}
