//! The worker protocol's data types. Each `serde` derive is the wire format;
//! the shell's TypeScript types are generated from it.
//! Optional fields keep `#[serde(default)]` although nothing deserializes yet: ADR 0010's
//! convention, so a DTO the shell later sends back accepts the field's absence.

mod code;
mod error;
mod locations;
mod proposal;

pub use code::BindingCode;
pub use error::EngineErrorDto;
