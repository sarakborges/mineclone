#import bevy_ui::ui_vertex_output UiVertexOutput

@group(1) @binding(0) var icon_texture: texture_2d<f32>;
@group(1) @binding(1) var icon_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(icon_texture, icon_sampler, in.uv);
}
