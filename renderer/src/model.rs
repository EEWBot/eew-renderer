#[derive(Debug)]
pub enum UserEvent {
    Shutdown,
    RenderingRequest((crate::RenderingContextV0, tokio::sync::oneshot::Sender<Vec<u8>>)),
}
