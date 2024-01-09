#[derive(Debug)]
pub enum EncodeContext {
    ScreenShot(usize),
    Web(tokio::sync::oneshot::Sender<Vec<u8>>),
}

#[derive(Debug)]
pub enum RedrawReason {
    Other,
    ScreenShot,
    Web(tokio::sync::oneshot::Sender<Vec<u8>>),
}

impl RedrawReason {
    pub fn is_buffer_needed(&self) -> bool {
        match self {
            Self::Other => false,
            Self::ScreenShot => true,
            Self::Web(_) => true,
        }
    }
}

impl Default for RedrawReason {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Default)]
pub struct RenderingContext {
    pub screenshot_count: usize,
}

#[derive(Debug)]
pub enum UserEvent {
    Shutdown,
    RenderingRequest(tokio::sync::oneshot::Sender<Vec<u8>>),
}
