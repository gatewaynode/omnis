//! The view's lines as Bevy meshes drawn through a material of the client's own (rendering
//! size 1, `tasks/plans/rendering-size-1.md`): [`mesh`] wraps a scene's [`Strokes`] in a
//! triangle list, and [`LineMaterial`] draws it with `lines.wesl`, which widens each segment's
//! quad to a width in pixels, lifts it off the fill it lies on and fades it with distance. The
//! mesh is built when the scene changes; nothing walks the segments each frame.

use crate::strokes::Strokes;
use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
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

/// The line material and its shader.
pub struct LinesPlugin;

impl Plugin for LinesPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "lines.wesl");
        app.add_plugins(MaterialPlugin::<LineMaterial>::default());
    }
}
