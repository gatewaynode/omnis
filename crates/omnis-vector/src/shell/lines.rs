//! The view's lines as Bevy meshes drawn through a material of the client's own (rendering
//! size 1, `tasks/plans/rendering-size-1.md`): [`mesh`] wraps a scene's [`Strokes`] in a
//! triangle list, and [`LineMaterial`] draws it with `lines.wesl`, which widens each segment's
//! quad to a width in pixels, lifts it off the fill it lies on and fades it with distance. The
//! mesh is built when the scene changes; nothing walks the segments each frame.

use crate::ground::Ground;
use crate::strokes::{AIM, CREST, Strokes};
use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    VertexFormat,
};
use bevy::shader::ShaderRef;

/// A vertex's other end of its segment; the first end is the mesh's position.
pub const ATTRIBUTE_END: MeshVertexAttribute =
    MeshVertexAttribute::new("LineEnd", 0x6f6d_6e69_0001, VertexFormat::Float32x3);
/// A vertex's corner of the quad: along (0 or 1) and side (-1 or +1).
pub const ATTRIBUTE_CORNER: MeshVertexAttribute =
    MeshVertexAttribute::new("LineCorner", 0x6f6d_6e69_0002, VertexFormat::Float32x2);
/// The line's linear colour, its level's brightness in the alpha.
pub const ATTRIBUTE_COLOUR: MeshVertexAttribute =
    MeshVertexAttribute::new("LineColour", 0x6f6d_6e69_0003, VertexFormat::Float32x4);
/// Weight, pattern, role and level.
pub const ATTRIBUTE_STYLE: MeshVertexAttribute =
    MeshVertexAttribute::new("LineStyle", 0x6f6d_6e69_0004, VertexFormat::Float32x4);

/// The material of a view's lines: where its fog ends, how wide a line is and how far it is
/// lifted toward the eye. One a view, shared by its line meshes.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct LineMaterial {
    /// `x`: the fog's end in cells; `y`: the width in pixels; `z`: the depth bias, negative
    /// toward the eye; `w`: unused.
    #[uniform(0)]
    pub params: Vec4,
}

impl LineMaterial {
    /// Lines that fade out at `fog_end` cells, `width` pixels wide, lifted by `depth_bias`.
    #[must_use]
    pub fn new(fog_end: f32, width: f32, depth_bias: f32) -> LineMaterial {
        LineMaterial {
            params: Vec4::new(fog_end, width, depth_bias, 0.0),
        }
    }

    /// Where the fog ends, in cells.
    #[must_use]
    pub fn fog_end(&self) -> f32 {
        self.params.x
    }

    /// The width in pixels.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.params.y
    }
}

/// The shader, compiled into the binary.
fn shader() -> ShaderRef {
    ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("lines.wesl")).with_source("embedded"))
}

impl Material for LineMaterial {
    fn vertex_shader() -> ShaderRef {
        shader()
    }

    fn fragment_shader() -> ShaderRef {
        shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    // The lines are neither in the depth prepass nor cast shadows; the default shaders of both
    // would not know this mesh's layout.
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_END.at_shader_location(1),
            ATTRIBUTE_CORNER.at_shader_location(2),
            ATTRIBUTE_COLOUR.at_shader_location(3),
            ATTRIBUTE_STYLE.at_shader_location(4),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        // A quad faces whichever way its segment runs across the screen.
        descriptor.primitive.cull_mode = None;
        // As the gizmo lines did: a line writes its depth and is drawn only where it is nearer
        // than what is there, so a near line is not painted over by a far one and, of two
        // lines in the same place, the first in the mesh shows.
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(true);
            depth.depth_compare = Some(CompareFunction::Greater);
        }
        Ok(())
    }
}

/// The lines as a mesh: a triangle list of quads, a vertex with both ends of its segment.
#[must_use]
pub fn mesh(strokes: &Strokes) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, strokes.a.clone())
    .with_inserted_attribute(ATTRIBUTE_END, strokes.b.clone())
    .with_inserted_attribute(ATTRIBUTE_CORNER, strokes.corner.clone())
    .with_inserted_attribute(ATTRIBUTE_COLOUR, strokes.colour.clone())
    .with_inserted_attribute(ATTRIBUTE_STYLE, strokes.style.clone())
    .with_inserted_indices(Indices::U32(strokes.indices.clone()))
}

/// What a line entity carries besides its mesh: the material, and no culling by its bounds,
/// which a mesh whose vertices all sit on their segment's first end does not describe.
#[must_use]
pub fn bundle(material: Handle<LineMaterial>) -> impl Bundle {
    (
        MeshMaterial3d(material),
        Transform::default(),
        NoFrustumCulling,
    )
}

/// The ground's crests seen from `eye` within `reach`, as lines.
#[must_use]
pub fn crests(ground: &Ground, eye: [f32; 3], reach: f32) -> Strokes {
    let mut strokes = Strokes::default();
    for (a, b) in ground.crests(eye, reach) {
        strokes.push(a, b, CREST, 1.0);
    }
    strokes
}

/// Where a view's crests were last drawn from, so they are found again only when the eye has
/// moved or the scene changed.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct CrestsAt {
    eye: Option<[f32; 3]>,
    reach: f32,
}

/// The crests of `ground` on `entity` as a mesh, when the eye has moved, the reach changed or
/// `fresh` says the ground did. The entity's mesh is replaced in place while there are crests,
/// and taken off when there are none: an empty mesh is nothing to upload.
pub fn hang_crests(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    (entity, held): (Entity, Option<&Mesh3d>),
    at: &mut CrestsAt,
    ground: &Ground,
    (eye, reach, fresh): ([f32; 3], f32, bool),
) {
    if !fresh && at.eye == Some(eye) && at.reach == reach {
        return;
    }
    *at = CrestsAt {
        eye: Some(eye),
        reach,
    };
    hang(
        commands,
        meshes,
        (entity, held),
        &crests(ground, eye, reach),
    );
}

/// How far inside its cell's sides the outline of an aimed floor runs, in cells.
pub const AIM_INSET: f32 = 0.125;

/// The outline of the floor of cell `(x, y)`, lit while the pointer is on the pile there
/// (owner, 2026-11-13): a square a little inside the cell, lying on the ground, in a thing's
/// colour. It shows where a click is the pile's.
#[must_use]
pub fn aim_mark(ground: &Ground, (x, y): (u16, u16)) -> Strokes {
    let (w, n) = (f32::from(x) + AIM_INSET, f32::from(y) + AIM_INSET);
    let side = 1.0 - 2.0 * AIM_INSET;
    let corners = [(w, n), (w + side, n), (w + side, n + side), (w, n + side)]
        .map(|(x, z)| [x, ground.height(x, z), z]);
    let mut strokes = Strokes::default();
    for i in 0..4 {
        strokes.push(corners[i], corners[(i + 1) % 4], AIM, 1.0);
    }
    strokes
}

/// `strokes` on `entity` as its mesh: replaced in place while there are lines, and taken off
/// when there are none, since an empty mesh is nothing to upload.
pub fn hang(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    (entity, held): (Entity, Option<&Mesh3d>),
    strokes: &Strokes,
) {
    match held {
        Some(_) if strokes.is_empty() => {
            commands.entity(entity).remove::<Mesh3d>();
        }
        Some(held) => {
            // The handle is the entity's own and alive, so the insert cannot fail.
            let _ = meshes.insert(held.0.id(), mesh(strokes));
        }
        None if strokes.is_empty() => {}
        None => {
            commands
                .entity(entity)
                .insert(Mesh3d(meshes.add(mesh(strokes))));
        }
    }
}

/// The line material and its shader.
pub struct LinesPlugin;

impl Plugin for LinesPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "lines.wesl");
        app.add_plugins(MaterialPlugin::<LineMaterial>::default());
    }
}
