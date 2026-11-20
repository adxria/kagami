//! Desktop-independent operations. Every mutation commits once, every workspace
//! read sees a single SQLite snapshot. Time is supplied by the adapter.
mod contexts;
mod data;
mod durable_fs;
mod recordings;
mod reporting;
mod store;
mod types;

pub use data::{DataImports, DataPreview, ImportData, ImportMode};
pub use store::Store;
pub use types::*;
