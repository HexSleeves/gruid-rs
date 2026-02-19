// Grid cell rendering shader.
//
// Each cell is an instanced quad. Per-instance data provides:
//   - grid position (col, row)
//   - foreground and background colors (packed u32)
//   - atlas UV rectangle for the glyph
//
// The vertex shader computes screen-space positions from grid coords
// and cell dimensions. The fragment shader samples the glyph atlas
// (single-channel alpha) and blends fg/bg colors.

struct Uniforms {
    // x: cell_width, y: cell_height, z: atlas_width, w: atlas_height
    cell_size: vec4<f32>,
    // x: grid pixel width, y: grid pixel height (for NDC conversion)
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_samp: sampler;

struct Instance {
    // x: col, y: row
    @location(0) grid_pos: vec2<f32>,
    // packed RGBA: R in bits 0-7, G 8-15, B 16-23, A 24-31
    @location(1) fg_color: u32,
    @location(2) bg_color: u32,
    // glyph atlas rect: x, y, w, h in texels
    @location(3) atlas_rect: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fg: vec4<f32>,
    @location(2) bg: vec4<f32>,
    // Whether this cell has a glyph (atlas_rect.z > 0)
    @location(3) has_glyph: f32,
};

// 4 vertices for a quad (triangle strip)
var<private> QUAD: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 1.0),
);

fn unpack_color(c: u32) -> vec4<f32> {
    let r = f32(c & 0xFFu) / 255.0;
    let g = f32((c >> 8u) & 0xFFu) / 255.0;
    let b = f32((c >> 16u) & 0xFFu) / 255.0;
    let a = f32((c >> 24u) & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    inst: Instance,
) -> VsOut {
    let corner = QUAD[vi];

    let cell_w = uniforms.cell_size.x;
    let cell_h = uniforms.cell_size.y;
    let atlas_w = uniforms.cell_size.z;
    let atlas_h = uniforms.cell_size.w;

    // Pixel position of this vertex
    let px = (inst.grid_pos.x + corner.x) * cell_w;
    let py = (inst.grid_pos.y + corner.y) * cell_h;

    // Convert to NDC: [-1, 1] with y-flip
    let ndc_x = (px / uniforms.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / uniforms.screen_size.y) * 2.0;

    // Atlas UV
    var uv = vec2<f32>(0.0, 0.0);
    var has_glyph = 0.0;
    if inst.atlas_rect.z > 0.0 {
        has_glyph = 1.0;
        let u0 = inst.atlas_rect.x / atlas_w;
        let v0 = inst.atlas_rect.y / atlas_h;
        let u1 = (inst.atlas_rect.x + inst.atlas_rect.z) / atlas_w;
        let v1 = (inst.atlas_rect.y + inst.atlas_rect.w) / atlas_h;
        uv = vec2<f32>(
            mix(u0, u1, corner.x),
            mix(v0, v1, corner.y),
        );
    }

    var out: VsOut;
    out.pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = uv;
    out.fg = unpack_color(inst.fg_color);
    out.bg = unpack_color(inst.bg_color);
    out.has_glyph = has_glyph;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if in.has_glyph < 0.5 {
        return in.bg;
    }
    let alpha = textureSample(atlas_tex, atlas_samp, in.uv).r;
    return mix(in.bg, in.fg, alpha);
}
