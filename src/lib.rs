//! # ts3
//! A WIP ts3 query interface library
//!
//! # Examples
//!
//! ```rust
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // Create a new client and connect to the server query interface
//!     let client = Client::new("localhost:10011").await?;
//!
//!     // switch to virtual server with id 1
//!     client.use_sid(1).await?;
//!
//!     Ok(())
//! }
//! ```

mod client;

pub use client::Client;
