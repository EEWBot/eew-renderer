pub fn capture<T>(device: &wgpu::Device, render: impl FnOnce() -> T) -> T {
    unsafe { device.start_graphics_debugger_capture() };

    let result = render();

    unsafe { device.stop_graphics_debugger_capture() };

    result
}
