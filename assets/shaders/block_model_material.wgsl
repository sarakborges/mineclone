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
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> block_model_material: BlockModelMaterialExtension;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let base_color = pbr_input.material.base_color;
    let maximum_channel = max(base_color.r, max(base_color.g, base_color.b));
    let minimum_channel = min(base_color.r, min(base_color.g, base_color.b));
    let chroma = maximum_channel - minimum_channel;

    if chroma <= 0.02 {
        let tint = clamp(block_model_material.tint.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        let tint_peak = max(max(tint.r, tint.g), max(tint.b, 0.001));
        let hue = tint / tint_peak;
        let softened_hue = mix(vec3<f32>(1.0), hue, 0.72);
        let luminance_weights = vec3<f32>(0.2126, 0.7152, 0.0722);
        let softened_luma = max(dot(softened_hue, luminance_weights), 0.001);
        let luminance_compensation = min(1.35, 1.0 / softened_luma);

        pbr_input.material.base_color = vec4<f32>(
            clamp(
                vec3<f32>(base_color.r)
                    * softened_hue
                    * luminance_compensation
                    * 1.08,
                vec3<f32>(0.0),
                vec3<f32>(1.0),
            ),
            base_color.a,
        );
    }

    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = pbr_input.material.base_color;
    return out;
#endif
}