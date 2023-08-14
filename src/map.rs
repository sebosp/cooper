//! Map related functionality.
use super::*;
use gloo_console::log;
use lyon::math::{point, Box2D, Point};
use lyon::path::{builder::BorderRadii, Winding};
use lyon::tessellation::geometry_builder::simple_builder;
use lyon::tessellation::{FillOptions, FillTessellator, VertexBuffers};

// Build a black background for the map.
pub fn build_map_background() -> Vec<f32> {
    let mut geometry: VertexBuffers<Point, u16> = VertexBuffers::new();
    let mut geometry_builder = simple_builder(&mut geometry);
    let options = FillOptions::tolerance(0.1);
    let mut tessellator = FillTessellator::new();

    let mut builder = tessellator.builder(&options, &mut geometry_builder);

    builder.add_rounded_rectangle(
        &Box2D {
            min: point(-1.0, -1.0),
            max: point(1.0, 1.0),
        },
        &BorderRadii {
            top_left: 0.02,
            top_right: 0.02,
            bottom_left: 0.02,
            bottom_right: 0.02,
        },
        Winding::Positive,
    );

    let _ = builder.build();

    // No idea how gl Draw Elements work so let's build the payload by hand:
    let mut vertices: Vec<f32> = Vec::with_capacity(geometry.indices.len() * 7usize);
    for idx in geometry.indices {
        vertices.push(geometry.vertices[idx as usize].x);
        vertices.push(geometry.vertices[idx as usize].y);
        vertices.push(0.0); // z
        vertices.push(0.0); // r
        vertices.push(0.0); // g
        vertices.push(0.0); // b
        vertices.push(1.0); // a
    }
    vertices
}
