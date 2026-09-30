//! Helium: the experimental obligation-concurrency library.
#![no_std]

mod model;
mod operation;
mod resource;

pub use model::{Error, Model, Stage};
pub use operation::{Operation, OperationId, State};
pub use resource::{Resource, ResourceId};

/// Version of this library crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
