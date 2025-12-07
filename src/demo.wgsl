struct PushConstants {
    redness: f32,
    resolution: vec2<f32>,
}
var<push_constant> pc: PushConstants;

@group(0) @binding(0) var output: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let uv = vec2<f32>(id.xy) / pc.resolution;
    let grid_uv = fract(uv * 8.0) - 0.5;
    let d = length(grid_uv) - 0.3;
    let c = 1.0 - smoothstep(0.0, 0.02, d);
    let color = vec3<f32>(c * pc.redness, c, c);
    textureStore(output, id.xy, vec4<f32>(color, 1.0));
}
