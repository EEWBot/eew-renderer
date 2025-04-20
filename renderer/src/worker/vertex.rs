use glium::implement_vertex;
use renderer_types::*;

#[derive(Copy, Clone, Debug)]
pub struct MapVertex {
    position: [f32; 2],
}

implement_vertex!(MapVertex, position);

impl MapVertex {
    pub fn new(position: Vertex<GeoDegree>) -> Self {
        Self { position: position.to_slice() }
    }
}

#[derive(Copy, Clone)]
pub struct え {
    position: [f32; 2],
    uv: [f32; 2],
}

impl え {
    pub fn new(position: (f32, f32), uv: (f32, f32)) -> Self {
        Self {
            position: [position.0, position.1],
            uv: [uv.0, uv.1],
        }
    }
}

implement_vertex!(え, position, uv);
