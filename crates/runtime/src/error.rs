use crate::Region;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuestError {
    #[error("Invalid flag value {0}")]
    InvalidFlagValue(&'static str),
    #[error("Invalid enum value {0}")]
    InvalidEnumValue(&'static str),
    #[error("Pointer out of bounds: {0:?}")]
    PtrOutOfBounds(Region),
    #[error("Pointer not aligned to {1}: {0:?}")]
    PtrNotAligned(Region, u32),
    #[error("Pointer already borrowed: {0:?}")]
    PtrBorrowed(Region),
    #[error("In func {funcname}:{location}:")]
    InFunc {
        funcname: &'static str,
        location: &'static str,
        #[source]
        err: Box<GuestError>,
    },
    #[error("In data {typename}.{field}:")]
    InDataField {
        typename: String,
        field: String,
        #[source]
        err: Box<GuestError>,
    },
    // FIXME the error should be more verbose and should print all valid_up_to chars
    #[error("Invalid UTF-8 encountered")]
    InvalidUtf8,
}
