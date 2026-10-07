//! Minimal GitHub REST client.
//!
//! Everything that needs the token runs in `client`; the token never reaches
//! the webview. Error messages are deliberately free of credential material.
//!
//! The split mirrors the direction data flows: `raw` is what GitHub sends,
//! `models` is what the webview receives, `status` and `dispatch` hold the
//! derivations in between, and `client` is the only part that does I/O.

mod client;
mod dispatch;
mod error;
mod models;
mod raw;
mod status;

pub use client::{GitHub, PULLS_PER_PAGE, RELEASES_PER_PAGE, RUNS_PER_PAGE};
pub use dispatch::parse_dispatch_inputs;
pub use error::ApiError;
pub use models::*;
