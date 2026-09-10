#import bevy_ui::ui_vertex_output UiVertexOutput

@group(1) @binding(0) var top_texture: texture_2d<f32>;
@group(1) @binding(1) var top_sampler: sampler;
@group(1) @binding(2) var front_texture: texture_2d<f32>;
@group(1) @binding(3) var front_sampler: sampler;
@group(1) @binding(4) var right_texture: texture_2d<f32>;
@group(1) @binding(5) var right_sampler: sampler;
@group(1) @binding(6) var<uniform> tint: vec4<f32>;
@group(1) @binding(7) var<uniform> face_shades: vec4<f32>;
@group(1) @binding(8) var<uniform> top_origin_axis_u: vec4<f32>;
@group(1) @binding(9) var<uniform> top_axis_v: vec4<f32>;
@group(1) @binding(10) var<uniform> front_origin_axis_u: vec4<f32>;
@group(1) @binding(11) var<uniform> front_axis_v: vec4<f32>;
@group(1) @binding(12) var<uniform> right_origin_axis_u: vec4<f32>;
@group(1) @binding(13) var<uniform> right_axis_v: vec4<f32>;

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

fn apply_biome_tint(texel: vec4<f32>) -> vec4<f32> {
    let maximum_channel = max(texel.r, max(texel.g, texel.b));
    let minimum_channel = min(texel.r, min(texel.g, texel.b));
    let chroma = maximum_channel - minimum_channel;

    if chroma > 0.02 {
        return texel;
    }

    let tint_rgb = clamp(tint.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let tint_peak = max(max(tint_rgb.r, tint_rgb.g), max(tint_rgb.b, 0.001));
    let hue = tint_rgb / tint_peak;
    let softened_hue = mix(vec3<f32>(1.0), hue, 0.72);
    let luminance_weights = vec3<f32>(0.2126, 0.7152, 0.0722);
    let softened_luma = max(dot(softened_hue, luminance_weights), 0.001);
    let luminance_compensation = min(1.35, 1.0 / softened_luma);
    let tinted = clamp(
        vec3<f32>(texel.r) * softened_hue * luminance_compensation * 1.08,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );

    return vec4<f32>(tinted, texel.a);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let point = in.uv;

    let top_uv = parallelogram_uv(point, top_origin_axis_u, top_axis_v);
    if inside_face(top_uv) {
        let sampled = textureSampleLevel(top_texture, top_sampler, top_uv, 0.0);
        let colored = apply_biome_tint(sampled);
        return vec4<f32>(colored.rgb * face_shades.x, colored.a);
    }

    let front_uv = parallelogram_uv(point, front_origin_axis_u, front_axis_v);
    if inside_face(front_uv) {
        let sampled = textureSampleLevel(front_texture, front_sampler, front_uv, 0.0);
        let colored = apply_biome_tint(sampled);
        return vec4<f32>(colored.rgb * face_shades.y, colored.a);
    }

    let right_uv = parallelogram_uv(point, right_origin_axis_u, right_axis_v);
    if inside_face(right_uv) {
        let sampled = textureSampleLevel(right_texture, right_sampler, right_uv, 0.0);
        let colored = apply_biome_tint(sampled);
        return vec4<f32>(colored.rgb * face_shades.z, colored.a);
    }

    return vec4<f32>(0.0);
}