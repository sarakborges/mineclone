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
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::main_pass_post_lighting_processing,
}
#endif

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let texel = textureSample(
        pbr_bindings::base_color_texture,
        pbr_bindings::base_color_sampler,
        in.uv,
    );
    let tint = clamp(in.color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let local_light = clamp(in.color.a, 0.0, 1.0);
    let maximum_channel = max(texel.r, max(texel.g, texel.b));
    let minimum_channel = min(texel.r, min(texel.g, texel.b));
    let chroma = maximum_channel - minimum_channel;

    var base_rgb = texel.rgb;
    if chroma <= 0.02 {
        // Grayscale pixels carry the authored brightness of the grass texture.
        // Use biome color only for hue/chroma, preserving that luminance instead of
        // multiplying the texture by a dark RGB tint.
        let tint_luma = max(
            dot(tint, vec3<f32>(0.2126, 0.7152, 0.0722)),
            0.001,
        );
        let luminance_preserving_tint = tint / tint_luma;
        base_rgb = clamp(
            vec3<f32>(texel.r) * luminance_preserving_tint,
            vec3<f32>(0.0),
            vec3<f32>(1.0),
        );
    }

    pbr_input.material.base_color = vec4<f32>(
        base_rgb * local_light * pbr_bindings::material.base_color.rgb,
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
