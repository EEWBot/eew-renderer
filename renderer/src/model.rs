use std::fmt::{Display, Formatter};

use enum_map::Enum;
use thiserror::Error;
use crate::worker::image_buffer::RGBAImageData;

#[derive(Error, Debug)]
pub enum RenderingError {}

#[allow(clippy::type_complexity)]
#[derive(Debug)]
pub enum Message {
    FrameRequest(
        (
            crate::frame_context::FrameContext,
            tokio::sync::oneshot::Sender<Result<RGBAImageData, RenderingError>>,
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
#[derive(Enum, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
#[repr(u8)]
pub enum 津波情報 {
    津波予報 = 1,
    津波注意報 = 2,
    津波警報 = 3,
    大津波警報 = 4,
}

impl Display for 津波情報 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            津波情報::津波予報 => f.write_str("津波予報(若干の海面変動)"),
            津波情報::津波注意報 => f.write_str("津波注意報"),
            津波情報::津波警報 => f.write_str("津波警報"),
            津波情報::大津波警報 => f.write_str("大津波警報"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TimeKind {
    発生,
    発表,
}

impl Display for TimeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeKind::発生 => f.write_str("発生"),
            TimeKind::発表 => f.write_str("発表"),
        }
    }
}
