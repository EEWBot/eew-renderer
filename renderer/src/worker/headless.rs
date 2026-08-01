use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::os::raw::c_void;
use std::rc::Rc;

use glium::backend::{Backend, Context};
use glium::glutin::api::egl::context::PossiblyCurrentContext;
use glium::glutin::api::egl::device::Device;
use glium::glutin::api::egl::display::Display;
use glium::glutin::api::egl::surface::Surface;
use glium::glutin::config::{ConfigSurfaceTypes, ConfigTemplateBuilder};
use glium::glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentGlContext, Version,
};
use glium::glutin::display::GlDisplay;
use glium::glutin::surface::{PbufferSurface, SurfaceAttributesBuilder};
use glium::SwapBuffersError;

struct HeadlessBackend {
    /// surfaceless なコンテキストを作れた場合は `None`。
    surface: Option<Surface<PbufferSurface>>,
    context: PossiblyCurrentContext,
    display: Display,
}

unsafe impl Backend for HeadlessBackend {
    fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        Ok(())
    }

    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
        let symbol = CString::new(symbol).unwrap();
        self.display.get_proc_address(&symbol)
    }

    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        (1, 1)
    }

    fn resize(&self, _: (u32, u32)) {}

    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    unsafe fn make_current(&self) {
        match &self.surface {
            Some(surface) => self.context.make_current(surface).unwrap(),
            None => self.context.make_current_surfaceless().unwrap(),
        }
    }
}

pub fn create(device_index: usize) -> Result<Rc<Context>, Box<dyn Error>> {
    let devices: Vec<Device> = Device::query_devices()
        .map_err(|e| format!("Failed to enumerate EGL devices (is libEGL installed?): {e}"))?
        .collect();

    if devices.is_empty() {
        return Err("No EGL device is available".into());
    }

    for (i, device) in devices.iter().enumerate() {
        tracing::info!(
            "EGL Device #{i}: {} ({})",
            device.name().unwrap_or("[unknown]"),
            device.vendor().unwrap_or("[unknown]"),
        );
    }

    let device = devices.get(device_index).ok_or_else(|| {
        format!(
            "EGL device #{device_index} is not available ({} device(s) found)",
            devices.len()
        )
    })?;

    let display = unsafe { Display::with_device(device, None) }
        .map_err(|e| format!("Failed to create an EGL display from the device: {e}"))?;

    let template = ConfigTemplateBuilder::new()
        .with_surface_type(ConfigSurfaceTypes::PBUFFER)
        .build();

    let config = unsafe { display.find_configs(template)? }
        .next()
        .ok_or("No EGL config is available")?;

    let attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 5))))
        .build(None);

    let context = unsafe { display.create_context(&config, &attributes)? };

    let (context, surface) = match context.make_current_surfaceless() {
        Ok(context) => {
            tracing::info!("EGL Surface: surfaceless");
            (context, None)
        }
        Err(e) => {
            tracing::info!("EGL Surface: 1x1 pbuffer (surfaceless is unavailable: {e})");

            let context = unsafe { display.create_context(&config, &attributes)? };

            let attributes = SurfaceAttributesBuilder::<PbufferSurface>::new()
                .build(NonZeroU32::new(1).unwrap(), NonZeroU32::new(1).unwrap());

            let surface = unsafe { display.create_pbuffer_surface(&config, &attributes)? };
            let context = context.make_current(&surface)?;

            (context, Some(surface))
        }
    };

    let backend = HeadlessBackend {
        surface,
        context,
        display,
    };

    Ok(unsafe { Context::new(backend, true, Default::default()) }?)
}
