use glium::backend::Facade;
use glium::index::IndicesSource;
use glium::uniforms::Uniforms;
use glium::{DrawError, DrawParameters, Program, Surface, Vertex, VertexBuffer};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ShaderProgram<U: Uniforms, V: Vertex> {
    program: Program,
    _uniform_type: PhantomData<U>,
    _vertex_type: PhantomData<V>,
}

impl<U: Uniforms, V: Vertex> ShaderProgram<U, V> {
    pub fn from_program(program: Program) -> Result<Self, ShaderInstantiationError> {
        Ok(Self {
            program,
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
            .map_err(ShaderInstantiationError::ProgramCreation)?;
        Self::from_program(program)
    }

    pub fn draw<'a, S: ?Sized + Surface, I: Into<IndicesSource<'a>>>(
        &self,
        surface: &mut S,
        vertex_buffer: &VertexBuffer<V>,
        index_buffer: I,
        uniform: &U,
        draw_parameters: &DrawParameters,
    ) -> Result<(), DrawError> {
        surface.draw(
            vertex_buffer,
            index_buffer,
            &self.program,
            uniform,
            draw_parameters,
        )
    }
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
