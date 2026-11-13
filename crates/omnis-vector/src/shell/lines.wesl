// The view's lines (rendering size 1): each segment is a quad whose vertices carry both ends.
// The vertex stage widens it to a constant width in pixels, after the technique of Bevy's gizmo
// line shader (bevy_gizmos_render `lines.wesl`, from
// https://wwwtyro.net/2019/11/18/instanced-lines.html), and lifts it toward the eye so a line is
// not hidden by the fill it lies on. The fragment stage fades it with its distance from the eye
// over the ground: full to half the fog's end, nothing at the end; and it feathers the line's
// two long edges over one pixel (rendering size 2, part 1), for which the quad is drawn half a
// pixel wider on each side than the line.

import bevy_pbr::render::mesh_view_bindings::view;

// x: where the fog ends, in cells. y: the line width in pixels. z: the depth bias (negative
// toward the eye). w: unused.
@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> params: vec4<f32>;

struct Vertex {
    @location(0) a: vec3<f32>,
    @location(1) b: vec3<f32>,
    // x: along the segment, 0 at a and 1 at b. y: the side, -1 or +1.
    @location(2) corner: vec2<f32>,
    // rgb: linear colour. a: the level's brightness.
    @location(3) colour: vec4<f32>,
    // x: weight, a factor on the width. y, z, w: pattern, role and level, kept for size 2.
    @location(4) style: vec4<f32>,
};

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) world: vec3<f32>,
    // x: how far this corner lies from the line's middle, in pixels, signed by its side.
    // y: half the line's width in pixels.
    @location(2) edge: vec2<f32>,
};

// The pixels beyond the line's own edge on each side that the quad covers, so the edge can fade.
const FEATHER: f32 = 0.5;

const EPSILON: f32 = 4.88e-04;

// How far from `a` toward `b` the near plane is crossed when `a` is behind it and `b` is not;
// 0 when `a` needs no move.
fn near_cut(a: vec4<f32>, b: vec4<f32>) -> f32 {
    if a.z > a.w && b.z <= b.w {
        let distance_a = a.z - a.w;
        let distance_b = b.z - b.w;
        return distance_a / (distance_a - distance_b) + EPSILON;
    }
    return 0.0;
}

@vertex
fn vertex(in: Vertex) -> Varying {
    let far_a = view.clip_from_world * vec4(in.a, 1.0);
    let far_b = view.clip_from_world * vec4(in.b, 1.0);

    // Each end moved to the near plane when it lies behind it, in clip space and in the world.
    let cut_a = near_cut(far_a, far_b);
    let cut_b = near_cut(far_b, far_a);
    let clip_a = mix(far_a, far_b, cut_a);
    let clip_b = mix(far_b, far_a, cut_b);
    let world_a = mix(in.a, in.b, cut_a);
    let world_b = mix(in.b, in.a, cut_b);

    let along = in.corner.x;
    let clip = mix(clip_a, clip_b, along);

    let resolution = view.viewport.zw;
    let screen_a = resolution * (0.5 * clip_a.xy / clip_a.w + 0.5);
    let screen_b = resolution * (0.5 * clip_b.xy / clip_b.w + 0.5);
    let forward = normalize(screen_b - screen_a);
    let across = vec2(-forward.y, forward.x);

    var colour = in.colour;
    var width = params.y * in.style.x;
    // A line thinner than a pixel is drawn a pixel wide and fainter.
    var thin = 1.0;
    if width > 0.0 && width < 1.0 {
        thin = width;
        width = 1.0;
    }

    let half = width * 0.5;
    let reach = half + FEATHER;
    let screen = mix(screen_a, screen_b, along) + reach * in.corner.y * across;

    // The gizmo shader's bias: depth * (w / depth)^-bias, toward the eye for a negative bias.
    var depth: f32;
    if params.z >= 0.0 {
        depth = clip.z * (1.0 - params.z);
    } else {
        depth = clip.z * exp2(-params.z * log2(clip.w / clip.z - EPSILON));
    }

    var out: Varying;
    out.clip = vec4(clip.w * ((2.0 * screen) / resolution - 1.0), depth, clip.w);
    out.colour = vec4(colour.rgb * thin, colour.a);
    out.world = mix(world_a, world_b, along);
    out.edge = vec2(reach * in.corner.y, half);
    return out;
}

@fragment
fn fragment(in: Varying) -> @location(0) vec4<f32> {
    let end = params.x;
    let start = end * 0.5;
    let distance = length(in.world.xz - view.world_position.xz);
    let fade = clamp(1.0 - (distance - start) / (end - start), 0.0, 1.0);
    // How much of this pixel the line covers across its width: all of it inside, none half a
    // pixel beyond its edge.
    let cover = clamp(in.edge.y + FEATHER - abs(in.edge.x), 0.0, 1.0);
    // Blended by alpha, as the gizmo lines were: the colour is scaled by the fade and the
    // level's brightness, and the fade is the alpha, here thinned by the cover.
    return vec4(in.colour.rgb * fade * in.colour.a, fade * cover);
}
