use enum_map::Enum;

#[derive(Debug)]
pub enum Message {
    RenderingRequest((crate::RenderingContextV0, tokio::sync::oneshot::Sender<Vec<u8>>)),
}

#[derive(Enum, Clone, Copy, Debug)]
#[repr(u8)]
pub enum 震度 {
    震度1,
    震度2,
    震度3,
    震度4,
    震度5弱,
    震度5強,
    震度6弱,
    震度6強,
    震度7,
}
