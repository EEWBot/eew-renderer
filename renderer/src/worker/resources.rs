use super::vertex::{BorderLineUniform, EpicenterUniform, EpicenterVertex, IntensityIconUniform, IntensityIconVertex, MapUniform, MapVertex, TextUniform, TextVertex, TexturedUniform, TexturedVertex};
use renderer_types::{GeoDegree, Vertex};

use glium::backend::Facade;
use glium::index::PrimitiveType;
use glium::texture::{MipmapsOption, RawImage2d, UncompressedFloatFormat};
use glium::{IndexBuffer, Texture2d, VertexBuffer};
use crate::worker::shader::ShaderProgram;

#[derive(Debug)]
pub struct Resources<'a> {
    pub shader: Shader<'a>,
    pub buffer: Buffer,
    pub lake: Lake,
    pub texture: Texture,
}

impl Resources<'_> {
    pub fn load<F: ?Sized + Facade>(facade: &F) -> Self {
        let shader = Shader::load(facade);
        let buffer = Buffer::load(facade);
        let lake = Lake::load(facade);
        let texture = Texture::load(facade);

        Self {
            shader,
            buffer,
            lake,
            texture,
        }
    }
}

#[derive(Debug)]
pub struct Buffer {
    pub map_vertex: VertexBuffer<MapVertex>,
    pub line_vertex: VertexBuffer<MapVertex>,
    area_line: Vec<IndexBuffer<u32>>,
    pref_line: Vec<IndexBuffer<u32>>,
    pub map: IndexBuffer<u32>,
}

impl Buffer {
    fn load<F: ?Sized + Facade>(facade: &F) -> Self {
        let geom = renderer_assets::QueryInterface::geometries();

        let map_vertices: Vec<_> = geom
            .map_vertices
            .iter()
            .map(|v| MapVertex {
                position: Vertex::<GeoDegree>::from((v.0, v.1)).to_slice()
            })
            .collect();

        let map_vertex = VertexBuffer::new(facade, &map_vertices).unwrap();

        let line_vertices: Vec<_> = geom
            .line_vertices
            .iter()
            .map(|v| MapVertex {
                position: Vertex::<GeoDegree>::from(*v).to_slice()
            })
            .collect();

        let line_vertex = VertexBuffer::new(facade, &line_vertices).unwrap();

        let map =
            IndexBuffer::new(facade, PrimitiveType::TrianglesList, geom.map_triangles).unwrap();

        let area_line: Vec<_> = geom
            .area_lines
            .iter()
            .map(|i| IndexBuffer::new(facade, PrimitiveType::LineStrip, i).unwrap())
            .collect();

        let pref_line = geom
            .pref_lines
            .iter()
            .map(|i| IndexBuffer::new(facade, PrimitiveType::LineStrip, i).unwrap())
            .collect();

        Buffer {
            map_vertex,
            line_vertex,
            map,
            area_line,
            pref_line,
        }
    }

    pub fn get_area_line_by_scale(&self, scale: f32) -> Option<&IndexBuffer<u32>> {
        let i = renderer_assets::QueryInterface::query_lod_level_by_scale(scale)?;
        self.area_line.get(i)
    }

    pub fn get_pref_line_by_scale(&self, scale: f32) -> Option<&IndexBuffer<u32>> {
        let i = renderer_assets::QueryInterface::query_lod_level_by_scale(scale)?;
        self.pref_line.get(i)
    }
}

#[derive(Debug)]
pub struct Lake {
    pub vertex: VertexBuffer<MapVertex>,
    pub index: IndexBuffer<u32>,
}

impl Lake {
    fn load<F: ?Sized + Facade>(facade: &F) -> Self {
        let geom = renderer_assets::QueryInterface::lake_geometries();

        let vertex: Vec<_> = geom
            .vertices
            .iter()
            .map(|v| MapVertex {
                position: Vertex::<GeoDegree>::from(*v).to_slice()
            })
            .collect();
        let vertex = VertexBuffer::immutable(facade, &vertex).unwrap();

        let index = IndexBuffer::immutable(facade, PrimitiveType::TrianglesList, geom.indices).unwrap();

        Lake {
            vertex,
            index,
        }
    }
}

#[derive(Debug)]
pub struct Shader<'a> {
    pub border_line: ShaderProgram<BorderLineUniform, MapVertex>,
    pub epicenter: ShaderProgram<EpicenterUniform<'a>, EpicenterVertex>,
    pub intensity_icon: ShaderProgram<IntensityIconUniform<'a>, IntensityIconVertex>,
    pub map: ShaderProgram<MapUniform, MapVertex>,
    pub text: ShaderProgram<TextUniform<'a>, TextVertex>,
    pub textured: ShaderProgram<TexturedUniform<'a>, TexturedVertex>,
}

impl Shader<'_> {
    fn load<F: ?Sized + Facade>(facade: &F) -> Self {
        let border_line = ShaderProgram::from_source(
            facade,
            include_str!("../../../assets/shader/border_line.vsh"),
            include_str!("../../../assets/shader/border_line.fsh"),
            Some(include_str!("../../../assets/shader/border_line.gsh")),
        )
        .unwrap();

        let epicenter = ShaderProgram::from_source(
            facade,
            include_str!("../../../assets/shader/epicenter.vsh"),
            include_str!("../../../assets/shader/epicenter.fsh"),
            Some(include_str!("../../../assets/shader/epicenter.gsh")),
        )
        .unwrap();

        let intensity_icon = ShaderProgram::from_source(
            facade,
            include_str!("../../../assets/shader/intensity_icon.vsh"),
            include_str!("../../../assets/shader/intensity_icon.fsh"),
            Some(include_str!("../../../assets/shader/intensity_icon.gsh")),
        )
        .unwrap();
        
        let map = ShaderProgram::from_source(
            facade,
            include_str!("../../../assets/shader/map.vsh"),
            include_str!("../../../assets/shader/map.fsh"),
            None,
        )
        .unwrap();

        let text = ShaderProgram::from_source(
            facade,
            include_str!("../../../assets/shader/text.vsh"),
            include_str!("../../../assets/shader/text.fsh"),
            None,
        )
        .unwrap();

        let textured = ShaderProgram::from_source(
            facade,
            include_str!("../../../assets/shader/textured.vsh"),
            include_str!("../../../assets/shader/textured.fsh"),
            None,
        )
        .unwrap();

        Self {
            border_line,
            epicenter,
            intensity_icon,
            map,
            text,
            textured,
        }
    }
}

#[derive(Debug)]
pub struct Texture {
    pub intensity: Texture2d,
    pub epicenter: Texture2d,
    pub overlay: Texture2d,
}

impl Texture {
    fn load<F: ?Sized + Facade>(facade: &F) -> Self {
        use image::ImageFormat;

        let load_png = |buf: &[u8]| -> Texture2d {
            let image = image::load_from_memory_with_format(buf, ImageFormat::Png).unwrap();
            let image = image.as_rgba8().unwrap();
            let dimension = image.dimensions();
            let image = RawImage2d::from_raw_rgba_reversed(image.as_raw(), dimension);

            Texture2d::with_format(
                facade,
                image,
                UncompressedFloatFormat::U8U8U8U8,
                MipmapsOption::NoMipmap,
            )
            .unwrap()
        };

        let intensity = load_png(include_bytes!("../../../assets/image/intensity.png"));
        let epicenter = load_png(include_bytes!("../../../assets/image/epicenter.png"));
        let overlay = load_png(include_bytes!("../../../assets/image/overlay.png"));

        Self {
            intensity,
            epicenter,
            overlay,
        }
    }
}
