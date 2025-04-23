use glium::implement_vertex;

#[derive(Copy, Clone)]
pub struct EpicenterVertex {
    pub position: [f32; 2],
}
implement_vertex!(EpicenterVertex, position);

#[derive(Copy, Clone)]
pub struct IntensityIconVertex {
    pub position: [f32; 2],
    pub uv_offset: [f32; 2],
}
implement_vertex!(IntensityIconVertex, position, uv_offset);

#[derive(Copy, Clone, Debug)]
pub struct MapVertex {
    pub position: [f32; 2],
}
implement_vertex!(MapVertex, position);

#[derive(Copy, Clone, Debug)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}
implement_vertex!(TextVertex, position, uv);

#[derive(Copy, Clone)]
pub struct TexturedVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}
implement_vertex!(TexturedVertex, position, uv);
