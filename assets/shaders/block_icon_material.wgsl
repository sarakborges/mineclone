#import bevy_ui::ui_vertex_output UiVertexOutput

@group(1) @binding(0) var top_texture: texture_2d<f32>;
@group(1) @binding(1) var top_sampler: sampler;
@group(1) @binding(2) var front_texture: texture_2d<f32>;
@group(1) @binding(3) var front_sampler: sampler;
@group(1) @binding(4) var right_texture: texture_2d<f32>;
@group(1) @binding(5) var right_sampler: sampler;
@group(1) @binding(6) var top_overlay_texture: texture_2d<f32>;
@group(1) @binding(7) var top_overlay_sampler: sampler;
@group(1) @binding(8) var front_overlay_texture: texture_2d<f32>;
@group(1) @binding(9) var front_overlay_sampler: sampler;
@group(1) @binding(10) var right_overlay_texture: texture_2d<f32>;
@group(1) @binding(11) var right_overlay_sampler: sampler;
@group(1) @binding(12) var<uniform> tint: vec4<f32>;
@group(1) @binding(13) var<uniform> face_shades: vec4<f32>;
@group(1) @binding(14) var<uniform> base_tint_flags: vec4<f32>;
@group(1) @binding(15) var<uniform> overlay_tint_flags: vec4<f32>;
@group(1) @binding(16) var<uniform> overlay_present_flags: vec4<f32>;
@group(1) @binding(17) var<uniform> top_origin_axis_u: vec4<f32>;
@group(1) @binding(18) var<uniform> top_axis_v: vec4<f32>;
@group(1) @binding(19) var<uniform> front_origin_axis_u: vec4<f32>;
@group(1) @binding(20) var<uniform> front_axis_v: vec4<f32>;
@group(1) @binding(21) var<uniform> right_origin_axis_u: vec4<f32>;
@group(1) @binding(22) var<uniform> right_axis_v: vec4<f32>;

const TINT_LUMINANCE_WEIGHTS: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

fn parallelogram_uv(
    point: vec2<f32>,
    origin_axis_u: vec4<f32>,
    axis_v: vec4<f32>,
) -> vec2<f32> {
    let origin = origin_axis_u.xy;
    let axis_u = origin_axis_u.zw;
    let face_axis_v = axis_v.xy;
    let delta = point - origin;
    let determinant = axis_u.x * face_axis_v.y - axis_u.y * face_axis_v.x;

    return vec2<f32>(
        (delta.x * face_axis_v.y - delta.y * face_axis_v.x) / determinant,
        (axis_u.x * delta.y - axis_u.y * delta.x) / determinant,
    );
}

fn inside_face(uv: vec2<f32>) -> bool {
    return uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
}

fn apply_layer_tint(texel: vec4<f32>, enabled: f32) -> vec4<f32> {
    if enabled <= 0.5 {
        return texel;
    }

    let source_luma = max(dot(texel.rgb, TINT_LUMINANCE_WEIGHTS), 0.0);
    let tint_rgb = clamp(tint.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let tint_peak = max(max(tint_rgb.r, tint_rgb.g), max(tint_rgb.b, 0.001));
    let hue = tint_rgb / tint_peak;
    let hue_luma = max(dot(hue, TINT_LUMINANCE_WEIGHTS), 0.001);
    let luminance_compensation = min(2.0, 1.0 / hue_luma);
    let tinted = clamp(
        vec3<f32>(source_luma) * hue * luminance_compensation * 1.08,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );

    return vec4<f32>(tinted, texel.a);
}

fn compose_layers(base: vec4<f32>, overlay: vec4<f32>, present: f32) -> vec4<f32> {
    if present <= 0.5 {
        return base;
    }

    let overlay_alpha = overlay.a;
    let out_alpha = overlay_alpha + base.a * (1.0 - overlay_alpha);
    if out_alpha <= 0.001 {
        return vec4<f32>(0.0);
    }

    let out_rgb = (
        overlay.rgb * overlay_alpha
        + base.rgb * base.a * (1.0 - overlay_alpha)
    ) / out_alpha;
    return vec4<f32>(out_rgb, out_alpha);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let point = in.uv;

    let top_uv = parallelogram_uv(point, top_origin_axis_u, top_axis_v);
    if inside_face(top_uv) {
        let base = apply_layer_tint(
            textureSampleLevel(top_texture, top_sampler, top_uv, 0.0),
            base_tint_flags.x,
        );
        var colored = base;
        if overlay_present_flags.x > 0.5 {
            let overlay = apply_layer_tint(
                textureSampleLevel(top_overlay_texture, top_overlay_sampler, top_uv, 0.0),
                overlay_tint_flags.x,
            );
            colored = compose_layers(base, overlay, overlay_present_flags.x);
        }
        return vec4<f32>(colored.rgb * face_shades.x, colored.a);
    }

    let front_uv = parallelogram_uv(point, front_origin_axis_u, front_axis_v);
    if inside_face(front_uv) {
        let base = apply_layer_tint(
            textureSampleLevel(front_texture, front_sampler, front_uv, 0.0),
            base_tint_flags.y,
        );
        var colored = base;
        if overlay_present_flags.y > 0.5 {
            let overlay = apply_layer_tint(
                textureSampleLevel(front_overlay_texture, front_overlay_sampler, front_uv, 0.0),
                overlay_tint_flags.y,
            );
            colored = compose_layers(base, overlay, overlay_present_flags.y);
        }
        return vec4<f32>(colored.rgb * face_shades.y, colored.a);
    }

    let right_uv = parallelogram_uv(point, right_origin_axis_u, right_axis_v);
    if inside_face(right_uv) {
        let base = apply_layer_tint(
            textureSampleLevel(right_texture, right_sampler, right_uv, 0.0),
            base_tint_flags.z,
        );
        var colored = base;
        if overlay_present_flags.z > 0.5 {
            let overlay = apply_layer_tint(
                textureSampleLevel(right_overlay_texture, right_overlay_sampler, right_uv, 0.0),
                overlay_tint_flags.z,
            );
            colored = compose_layers(base, overlay, overlay_present_flags.z);
        }
        return vec4<f32>(colored.rgb * face_shades.z, colored.a);
    }

    return vec4<f32>(0.0);
}
