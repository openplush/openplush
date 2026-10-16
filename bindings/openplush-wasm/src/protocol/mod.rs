//! The worker protocol's data types. Each `serde` derive is the wire format;
//! the shell's TypeScript types are generated from it.
//! Optional fields keep `#[serde(default)]`, ADR 0010's convention, so a DTO the shell
//! sends back accepts the field's absence.

mod code;
mod envelope;
mod error;
mod handle;
mod locations;
mod progress;
mod proposal;
mod protocol_error;

pub use code::BindingCode;
pub use envelope::{
    Failure, Op, ProgressUpdate, Request, RequestId, Response, Success, WarningDto,
};
pub use error::EngineErrorDto;
pub use handle::{MeshHandle, PipelineHandle};
pub use progress::{Progress, Working};
pub use protocol_error::ProtocolError;
