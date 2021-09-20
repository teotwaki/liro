pub mod auth;
mod client;
mod error;

pub use error::Error;
use error::Result;

pub use client::Client;
