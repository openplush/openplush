//! The worker protocol's data types. Each `serde` derive is the wire format;
//! the shell's TypeScript types are generated from it.

mod code;
mod error;
mod locations;
mod proposal;

pub use code::BindingCode;
pub use error::EngineErrorDto;
