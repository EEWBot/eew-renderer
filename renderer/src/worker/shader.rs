use glium::backend::Facade;
use glium::uniforms::Uniforms;
use glium::{Program, Surface, Vertex};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;

pub struct ShaderProgram<T: Uniforms, U: ToUniform<T>, V: Vertex> {
    program: Program,
    _uniforms: PhantomData<T>,
    _uniform_type: PhantomData<U>,
    _vertex_type: PhantomData<V>,
}
impl<T: Uniforms, U: ToUniform<T>, V: Vertex> ShaderProgram<T, U, V> {
    pub fn from_program(program: Program) -> Result<Self, ShaderInstantiationError> {
        Ok(Self {
            program,
            _uniforms: PhantomData,
            _uniform_type: PhantomData,
            _vertex_type: PhantomData,
        })
    }

    pub fn from_source<F: ?Sized + Facade>(
        facade: &F,
        vertex_shader: &str,
        fragment_shader: &str,
        geometry_shader: Option<&str>,
    ) -> Result<Self, ShaderInstantiationError> {
        let program = Program::from_source(facade, vertex_shader, fragment_shader, geometry_shader)
            .map_err(|e| ShaderInstantiationError::ProgramCreation(e))?;
        Self::from_program(program)
    }
}

pub trait ToUniform<T: Uniforms> {
    fn to_uniform(&self) -> &T;
}

#[derive(Debug)]
pub enum ShaderInstantiationError {
    ProgramCreation(glium::program::ProgramCreationError),
}

impl Display for ShaderInstantiationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderInstantiationError::ProgramCreation(e) => {
                write!(f, "Failed to create program: {e}")
            }
        }
    }
}

impl std::error::Error for ShaderInstantiationError {}

pub mod draw {
    use crate::worker::shader::{ShaderProgram, ToUniform};
    use glium::framebuffer::SimpleFrameBuffer;
    use glium::index::IndicesSource;
    use glium::uniforms::Uniforms;
    use glium::{DrawError, DrawParameters, Surface, Vertex, VertexBuffer};

    pub trait ShaderProgramDraw {
        fn draw<'a, T: Uniforms, U: ToUniform<T>, V: Vertex, I: Into<IndicesSource<'a>>>(
            &mut self,
            shader: &ShaderProgram<T, U, V>,
            vertex: &VertexBuffer<V>,
            index_buffer: I,
            uniform: &U,
            draw_parameters: &DrawParameters,
        ) -> Result<(), DrawError>;
    }

    impl<'a> ShaderProgramDraw for SimpleFrameBuffer<'a> {
        fn draw<'b, T: Uniforms, U: ToUniform<T>, V: Vertex, I: Into<IndicesSource<'b>>>(
            &mut self,
            shader: &ShaderProgram<T, U, V>,
            vertex: &VertexBuffer<V>,
            index_buffer: I,
            uniform: &U,
            draw_parameters: &DrawParameters,
        ) -> Result<(), DrawError> {
            Surface::draw(
                self,
                vertex,
                index_buffer,
                &shader.program,
                uniform.to_uniform(),
                draw_parameters,
            )
        }
    }
}
