use thiserror::Error;
use strum_macros::Display;

#[derive(Display, Debug, Clone, Copy)]
pub enum VersionedTypeId {
    QuakePrefectureV0,
    TsunamiForecastV0,
    TsunamiForecastV1,
}

#[derive(Error, Debug, Clone, Copy)]
pub enum VersionedTypeIdError {
    #[error("Unknown VersionedTypeId {0}")]
    UnknownVersionedTypeId(u8),
}

impl TryFrom<u8> for VersionedTypeId {
    type Error = VersionedTypeIdError;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            0 => Ok(Self::QuakePrefectureV0),
            1 => Ok(Self::TsunamiForecastV0),
            2 => Ok(Self::TsunamiForecastV1),
            _ => Err(VersionedTypeIdError::UnknownVersionedTypeId(version)),
        }
    }
}

impl VersionedTypeId {
    pub fn is_legacy_format_allowed(&self) -> bool {
        matches!(self, VersionedTypeId::QuakePrefectureV0)
    }
}
