//! # TS3
//! A fully asynchronous library to interact with the TeamSpeak 3 Server query interface.
//! The commands are avaliable after connecting to a TS3 Server using a [`Client`]. Commands
//! can either be sent using the associated command or using [`Client.sent`] to send raw messages.
//!
//! # Examples
//!
//! Connect to a TS3 query interface and select a server
//! ```no_run
//! use ts3::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // Create a new client and connect to the server query interface
//!     let client = Client::connect("localhost:10011").await?;
//!
//!     // switch to virtual server with id 1
//!     client.use_sid(1).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ```no_run
//! use ts3::{Client, async_trait};
//! use ts3::request::{TextMessageTarget};
//! use ts3::event::{EventHandler, ClientEnterView};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let client = Client::connect("localhost:10011").await?;
//!
//!     client.use_sid(1).await?;
//!
//!     // Assign a new event handler.
//!     client.set_event_handler(Handler);
//!
//!     tokio::signal::ctrl_c().await?;
//!     Ok(())
//! }
//!
//! pub struct Handler;
//!
//! #[async_trait]
//! impl EventHandler for Handler {
//!     async fn cliententerview(&self, client: Client, event: ClientEnterView) {
//!         println!("Client {} joined!", event.client_nickname);
//!
//!         // Send a private message to the client using "sendtextmessage".
//!         client.sendtextmessage(TextMessageTarget::Client(event.clid), "Hello World!")
//!             .await.unwrap();
//!     }
//! }
//!
//! ```

extern crate self as ts3;

mod client;
pub mod event;
pub mod request;
pub mod response;
pub mod shared;
mod types;

pub use async_trait::async_trait;
pub use client::Client;
pub use ts3_derive::Decode;

use std::{
    convert::{Infallible, TryFrom},
    fmt::{Debug, Write},
    io,
    num::ParseIntError,
    str::{from_utf8, Utf8Error},
};

use thiserror::Error;

/// An error that can occur when interacting with the TS3 query API.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl From<Infallible> for Error {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, Error)]
enum ErrorKind {
    /// Error returned from the ts3 interface. id of 0 indicates no error.
    #[error("TS3 error {id}: {msg}")]
    TS3 { id: u16, msg: String },
    /// Io error from the underlying tcp stream.
    #[error("io: {0}")]
    Io(#[from] io::Error),
    /// Error occured while decoding the server response.
    #[error("failed to decode stream: {0}")]
    Decode(#[from] DecodeError),
    #[error("failed to parse integer: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("recevied invalid utf8: {0}")]
    Utf8(#[from] Utf8Error),
    #[error("send error")]
    SendError,
    #[error("no field")]
    NoField,
}

#[derive(Debug, Error)]
enum DecodeError {
    #[error("unexpected eof")]
    UnexpectedEof,
    #[error("unexpected byte: {0}")]
    UnexpectedByte(u8),
    #[error("invalid reasonid: {0}")]
    InvalidReasonId(u8),
    #[error("invalid apikey scope: {0}")]
    InvalidApiKeyScope(String),
}

/// Any type implementing `Decode` can be directly decoded from the TS3 stream.
/// It provides the complete buffer of the response from the stream.
pub trait Decode: Sized {
    type Error: std::error::Error;

    fn decode(buf: &[u8]) -> Result<Self, Self::Error>;
}

pub trait Encode {
    fn encode(&self, buf: &mut String);
}

/// Implements `Serialize` for types that can be directly written as they are formatted.
macro_rules! impl_serialize {
    ($t:ty) => {
        impl crate::Encode for $t {
            fn encode(&self, writer: &mut ::std::string::String) {
                write!(writer, "{}", self).unwrap();
            }
        }
    };
}

/// The `impl_decode` macro implements `Decode` for any type that implements `FromStr`.
macro_rules! impl_decode {
    ($t:ty) => {
        impl Decode for $t {
            type Error = Error;

            fn decode(buf: &[u8]) -> std::result::Result<$t, Self::Error> {
                Ok(from_utf8(buf)
                    .map_err(|e| Error(ErrorKind::Utf8(e)))?
                    .parse()
                    .map_err(|e| Error(ErrorKind::ParseInt(e)))?)
            }
        }
    };
}

/// Implement `Decode` for `()`. Calling `()::decode(&[u8])` will never fail
/// and can always be unwrapped safely.
impl Decode for () {
    type Error = Infallible;

    fn decode(_: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}

// Implement `Decode` for `String`
impl Decode for String {
    type Error = Error;

    fn decode(buf: &[u8]) -> Result<String, Self::Error> {
        // Create a new string, allocating the same length as the buffer. Most
        // chars are one-byte only.
        let mut string = String::with_capacity(buf.len());

        // Create a peekable iterator to iterate over all bytes, appending all bytes
        // and replacing escaped chars.
        let mut iter = buf.into_iter().peekable();
        while let Some(b) = iter.next() {
            match b {
                // Match any escapes, starting with a '\' followed by another char.
                b'\\' => {
                    match iter.peek() {
                        Some(c) => match c {
                            b'\\' => string.push('\\'),
                            b'/' => string.push('/'),
                            b's' => string.push(' '),
                            b'p' => string.push('|'),
                            b'a' => string.push(7u8 as char),
                            b'b' => string.push(8u8 as char),
                            b'f' => string.push(12u8 as char),
                            b'n' => string.push(10u8 as char),
                            b'r' => string.push(13u8 as char),
                            b't' => string.push(9u8 as char),
                            b'v' => string.push(11u8 as char),
                            _ => {
                                return Err(Error(ErrorKind::Decode(DecodeError::UnexpectedByte(
                                    **c,
                                ))))
                            }
                        },
                        None => {
                            return Err(Error(ErrorKind::Decode(DecodeError::UnexpectedEof.into())))
                        }
                    }
                    iter.next();
                }
                _ => string.push(char::try_from(*b).unwrap()),
            }
        }

        // Shrink the string to its fitting size before returning it.
        string.shrink_to_fit();
        Ok(string)
    }
}

impl Encode for &str {
    fn encode(&self, writer: &mut String) {
        for c in self.chars() {
            match c {
                '\\' => writer.write_str("\\\\").unwrap(),
                '/' => writer.write_str("\\/").unwrap(),
                ' ' => writer.write_str("\\s").unwrap(),
                '|' => writer.write_str("\\p").unwrap(),
                c if c == 7u8 as char => writer.write_str("\\a").unwrap(),
                c if c == 8u8 as char => writer.write_str("\\b").unwrap(),
                c if c == 12u8 as char => writer.write_str("\\f").unwrap(),
                c if c == 10u8 as char => writer.write_str("\\n").unwrap(),
                c if c == 13u8 as char => writer.write_str("\\r").unwrap(),
                c if c == 9u8 as char => writer.write_str("\\t").unwrap(),
                c if c == 11u8 as char => writer.write_str("\\v").unwrap(),
                _ => writer.write_char(c).unwrap(),
            }
        }
    }
}

impl Encode for bool {
    fn encode(&self, writer: &mut String) {
        write!(
            writer,
            "{}",
            match self {
                false => b'0',
                true => b'1',
            }
        )
        .unwrap();
    }
}

impl Decode for bool {
    type Error = Error;

    fn decode(buf: &[u8]) -> Result<bool, Self::Error> {
        match buf.get(0) {
            Some(b) => match b {
                b'0' => Ok(true),
                b'1' => Ok(false),
                _ => Err(Error(ErrorKind::Decode(
                    DecodeError::UnexpectedByte(*b).into(),
                ))),
            },
            None => Err(Error(ErrorKind::Decode(DecodeError::UnexpectedEof.into()))),
        }
    }
}

// Implement `Decode` for all int types.
impl_decode!(isize);
impl_decode!(i8);
impl_decode!(i16);
impl_decode!(i32);
impl_decode!(i64);
impl_decode!(i128);

impl_decode!(usize);
impl_decode!(u8);
impl_decode!(u16);
impl_decode!(u32);
impl_decode!(u64);
impl_decode!(u128);

impl_serialize!(isize);
impl_serialize!(i8);
impl_serialize!(i16);
impl_serialize!(i32);
impl_serialize!(i64);
impl_serialize!(i128);

impl_serialize!(usize);
impl_serialize!(u8);
impl_serialize!(u16);
impl_serialize!(u32);
impl_serialize!(u64);
impl_serialize!(u128);

impl Error {
    fn decode(buf: &[u8]) -> Result<Error, Error> {
        let (mut id, mut msg) = (0, String::new());

        // Error is a key-value map separated by ' ' with only the id and msg key.
        for s in buf.split(|c| *c == b' ') {
            // Error starts with "error" as the first key.
            if s == b"error" {
                continue;
            }

            // Get both key and value from the buffer, separated by a '='.
            let parts: Vec<&[u8]> = s.splitn(2, |c| *c == b'=').collect();

            match parts.get(0) {
                Some(key) => {
                    // Extract the value.
                    let val = match parts.get(1) {
                        Some(val) => val,
                        None => return Err(Error(ErrorKind::Decode(DecodeError::UnexpectedEof))),
                    };

                    // Match the key of the pair and assign the corresponding value.
                    match *key {
                        b"id" => {
                            id = u16::decode(val)?;
                        }
                        b"msg" => {
                            msg = String::decode(val)?;
                        }
                        _ => (),
                    }
                }
                None => return Err(Error(ErrorKind::Decode(DecodeError::UnexpectedEof))),
            }
        }

        Ok(Error(ErrorKind::TS3 { id, msg }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Decode, Error, ErrorKind};

    #[test]
    fn test_string_decode() {
        let buf = b"Hello\\sWorld!";
        assert_eq!(String::decode(buf).unwrap(), "Hello World!".to_owned());
    }

    #[test]
    fn test_error_decode() {
        let buf = b"error id=0 msg=ok";
        let (id, msg) = match Error::decode(buf).unwrap().0 {
            ErrorKind::TS3 { id, msg } => (id, msg),
            _ => unreachable!(),
        };
        assert!(id == 0 && msg == "ok".to_owned());
    }
}
