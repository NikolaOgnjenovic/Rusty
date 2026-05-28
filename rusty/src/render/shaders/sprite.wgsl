struct VsIn {
    @location(0) vertex_pos: vec2<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) world_pos: vec2<f32>,
    @location(3) rotation: f32,
    @location(4) scale: vec2<f32>,
    @location(5) draw_size: vec2<f32>,
    @location(6) uv_rect: vec4<f32>,
    @location(7) tint: vec4<f32>,
    @location(8) z: i32,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

struct Camera {
    camera_pos: vec2<f32>,
    zoom: f32,
    _pad0: f32,
    viewport: vec2<f32>,
    _pad1: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;

    let scaled_size = in.draw_size * in.scale;
    let local = in.vertex_pos * scaled_size;
    let pivot = scaled_size * 0.5;
    let centered_local = local - pivot;
    let s = sin(in.rotation);
    let c = cos(in.rotation);
    let rotated = vec2<f32>(
        centered_local.x * c - centered_local.y * s,
        centered_local.x * s + centered_local.y * c,
    );

    let world = in.world_pos + rotated + pivot;

    let left = camera.camera_pos.x;
    let right = camera.camera_pos.x + camera.viewport.x / camera.zoom;
    let top = camera.camera_pos.y;
    let bottom = camera.camera_pos.y + camera.viewport.y / camera.zoom;

    let ndc_x = ((world.x - left) / (right - left)) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((world.y - top) / (bottom - top)) * 2.0;
    let clip = vec2<f32>(ndc_x, ndc_y);
    out.clip_pos = vec4<f32>(clip, f32(in.z) * 0.000001, 1.0);

    out.uv = in.uv_rect.xy + in.vertex_uv * in.uv_rect.zw;
    out.tint = in.tint;
    return out;
}

@group(0) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(0) @binding(1)
var sprite_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, in.uv) * in.tint;
}
