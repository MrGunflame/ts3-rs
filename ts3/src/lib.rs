//! # ts3
//! A WIP ts3 query interface library
//!
//! # Examples
//!
//! ```rust
//! use ts3::Client;
//!
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

pub mod client;
pub mod event;
mod macros;

pub use client::{Client, RawResp};
pub use event::EventHandler;
pub use ts3_derive::Decode;

use std::convert::TryFrom;
use std::fmt::Debug;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::str::{from_utf8, FromStr};
use std::error;

pub enum ParseError {
    InvalidEnum,
}

type BoxError = Box<dyn error::Error + Sync + Send>;

#[derive(Debug)]
pub enum Error {
    /// Error returned from the ts3 interface. id of 0 indicates no error.
    TS3 { id: u16, msg: String },
    /// Io error from the underlying tcp stream.
    Io(io::Error),
    /// Error occured while decoding the server response.
    Decode(BoxError),
    SendError,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::TS3 { id, msg } => write!(f, "TS3 Error {}: {}", id, msg),
            Self::Io(err) => write!(f, "Io Error: {}", err),
            Self::Decode(err) => write!(f, "Error decoding value: {}", err),
            Self::SendError => write!(f, "Failed to send command, channel closed"),
        }
    }
}

impl error::Error for Error {}

#[derive(Debug)]
pub enum DecodeError {
    UnexpectedEof,
    UnexpectedChar(char),
    UnexpectedByte(u8),
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "Unexpected end while decoding"),
            Self::UnexpectedChar(c) => write!(f, "Unexpected char decoding: {}", c),
            Self::UnexpectedByte(b) => write!(f, "Unexpected byte decoding: {}", b),
        }
    }
}

impl error::Error for DecodeError {}

/// A list of other objects that are being read from or written to the TS3 server interface.
/// It implements both `FromStr` and `ToString` as long as `T` itself also implements these traits.
#[derive(Debug, PartialEq)]
pub struct List<T> {
    items: Vec<T>,
}

impl<T> List<T> {
    /// Create a new empty list
    pub fn new() -> List<T> {
        List { items: Vec::new() }
    }

    /// Create a new list filled with the items in the `Vec`.
    pub fn from_vec(vec: Vec<T>) -> List<T> {
        List { items: vec }
    }

    /// Push an item to the end of the list
    fn push(&mut self, item: T) {
        self.items.push(item);
    }

    /// Consumes the List and returns the inner `Vec` of all items in the list.
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<T> FromStr for List<T>
where
    T: FromStr,
{
    type Err = <T as FromStr>::Err;

    fn from_str(s: &str) -> Result<List<T>, Self::Err> {
        let parts: Vec<&str> = s.split("|").collect();

        let mut list = List::new();
        for item in parts {
            match T::from_str(&item) {
                Ok(item) => list.push(item),
                Err(err) => return Err(err),
            }
        }

        Ok(list)
    }
}

impl<T> ToString for List<T>
where
    T: ToString,
{
    fn to_string(&self) -> String {
        match self.items.len() {
            0 => "".to_owned(),
            1 => self.items[0].to_string(),
            _ => {
                let mut string = String::new();
                string.push_str(&self.items[0].to_string());
                for item in &self.items[1..] {
                    string.push('|');
                    string.push_str(&item.to_string());
                }

                string
            }
        }
    }
}

/// Any type implementing `Decode` can be directly decoded from the TS3 stream.
/// It provides the complete buffer of the response from the stream.
pub trait Decode<T> {
    fn decode(buf: &[u8]) -> Result<T, BoxError>;
}

/// Implement `Decode` for `Vec` as long as `T` itself also implements `Decode`.
impl<T> Decode<Vec<T>> for Vec<T>
where
    T: Decode<T>,
{
    fn decode(buf: &[u8]) -> Result<Vec<T>, BoxError> {
        // Create a new vec and push all items to it.
        // Items are separated by a '|' char and no space before/after.
        let mut list = Vec::new();
        for b in buf.split(|c| *c == b'|') {
            list.push(T::decode(&b)?);
        }
        Ok(list)
    }
}

/// The `impl_decode` macro implements `Decode` for any type that implements `FromStr`.
macro_rules! impl_decode {
    ($t:ty) => {
        impl Decode<$t> for $t {
            fn decode(buf: &[u8]) -> std::result::Result<$t, BoxError> {
                Ok(from_utf8(buf)?.parse()?)
            }
        }
    };
}

/// Implement `Decode` for `()`. Calling `()::decode(&[u8])` will never fail
/// and can always be unwrapped safely.
impl Decode<()> for () {
    fn decode(_: &[u8]) -> Result<(), BoxError> {
        Ok(())
    }
}

// Implement `Decode` for `String`
impl Decode<String> for String {
    fn decode(buf: &[u8]) -> Result<String, BoxError> {
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
                            _ => return Err(Box::new(DecodeError::UnexpectedByte(**c))),
                        },
                        None => return Err(Box::new(DecodeError::UnexpectedEof)),
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

impl Decode<bool> for bool {
    fn decode(buf: &[u8]) -> Result<bool, BoxError> {
        match buf.get(0) {
            Some(b) => match b {
                b'0' => Ok(true),
                b'1' => Ok(false),
                _ => Err(Box::new(DecodeError::UnexpectedByte(*b))),
            },
            None => Err(Box::new(DecodeError::UnexpectedEof)),
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

impl Decode<Error> for Error {
    fn decode(buf: &[u8]) -> Result<Error, BoxError> {
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
                        None => return Err(Box::new(DecodeError::UnexpectedEof)),
                    };

                    // Match the key of the pair and assign the corresponding value.
                    match *key {
                        b"id" => {
                            id = u16::decode(val)?;
                        },
                        b"msg" => {
                            msg = String::decode(val)?;
                        }
                        _ => (),
                    }
                }
                None => return Err(Box::new(DecodeError::UnexpectedEof)),
            }
        }

        Ok(Error::TS3 { id, msg })
    }
}

mod tests {
    use std::str::FromStr;
    use super::*;

    #[test]
    fn test_vec_decode() {
        let buf = b"test|test2";
        assert_eq!(Vec::<String>::decode(buf).unwrap(), vec!["test".to_owned(), "test2".to_owned()]);
    }

    #[test]
    fn test_string_decode() {
        let buf = b"Hello\\sWorld!";
        assert_eq!(String::decode(buf).unwrap(), "Hello World!".to_owned());
    }

    #[test]
    fn test_error_decode() {
        let buf = b"error id=0 msg=ok";
        let (id, msg) = match Error::decode(buf).unwrap() {
            Error::TS3 { id, msg } => (id, msg),
            _ => unreachable!(),
        };
        assert!(id == 0 && msg == "ok".to_owned());
    }

    #[test]
    fn test_list_to_string() {
        let mut list = List::new();
        assert_eq!(list.to_string(), "");
        list.push(1);
        assert_eq!(list.to_string(), "1");
        list.push(2);
        assert_eq!(list.to_string(), "1|2");
    }

    #[test]
    fn test_list_from_str() {
        let string = "1|2|3|4";
        assert_eq!(
            List::from_str(&string).unwrap(),
            List {
                items: vec![1, 2, 3, 4]
            }
        );
    }
}
