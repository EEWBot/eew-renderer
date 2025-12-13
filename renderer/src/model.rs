use enum_map::Enum;

#[derive(Debug)]
pub enum Message {
    RenderingRequest(
        (
            crate::rendering_context::RenderingContext,
            tokio::sync::oneshot::Sender<Vec<u8>>,
        ),
    ),
}

#[allow(clippy::enum_variant_names)]
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

#[allow(clippy::enum_variant_names)]
#[derive(Enum, Clone, Copy, Debug)]
#[repr(u8)]
pub enum 津波情報 {
    津波予報 = 1,
    津波注意報 = 2,
    津波警報 = 3,
    大津波警報 = 4,
}
