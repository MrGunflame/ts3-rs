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

pub use client::{Client, RawResp};
pub use event::EventHandler;

use std::str::{FromStr, from_utf8};
use std::string::FromUtf8Error;
use std::num::ParseIntError;
use std::fmt::Debug;
use std::io;
use std::fmt::{self, Formatter, Display};
pub use ts3_derive::Decode;

pub enum ParseError {
    InvalidEnum,
}

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

mod tests {
    use super::List;
    use std::str::FromStr;

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

/// A type implementing `Decode` allows to be read from the TS stream
pub trait Decode<T> {
    type Err: Debug;

    fn decode(buf: &[u8]) -> Result<T, Self::Err>;
}

// Implement `Decode` for `Vec<T>` if T implements `Decode`
impl<T> Decode<Vec<T>> for Vec<T>
where T: Decode<T> {
    type Err = T::Err;

    fn decode(buf: &[u8]) -> Result<Vec<T>, Self::Err> {
        // Create a new vec and push all items to it
        // Items are separated by a '|' char and no space before/after
        let mut list = Vec::new();
        for b in buf.split(|c|*c == b'|') {
            list.push(T::decode(&b)?);
        }
        Ok(list)
    }
}

/// The `impl_decode` macro implements `Decode` for any type that implements `FromStr`.
#[macro_export]
macro_rules! impl_decode {
    ($t:ty) => {
        impl Decode<$t> for $t {
            type Err = std::num::ParseIntError;

            fn decode(buf: &[u8]) -> std::result::Result<$t, Self::Err> {
                from_utf8(buf).unwrap().parse()
            }
        }
    };
}

// Implement `Decode` for `()`. Calling `()::decode(&[u8])` will never fail.
impl Decode<()> for () {
    type Err = ();

    fn decode(_: &[u8]) -> Result<(), ()> {
        Ok(())
    }
}

// Implement `Decode` for `String`
impl Decode<String> for String {
    type Err = std::string::FromUtf8Error;

    fn decode(buf: &[u8]) -> Result<String, Self::Err> {
        String::from_utf8(buf.to_vec())
    }
}

// Implement all integer types
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

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    TS3 { id: u16, msg: String },
    SendError,
    ParseIntError(ParseIntError),
    Utf8Error(FromUtf8Error),
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        Error::ParseIntError(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Error {
        Error::Utf8Error(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Error::*;
        write!(
            f,
            "{}",
            match self {
                IO(err) => format!("{}", err),
                TS3 { id, msg } => format!("TS3 Error {}: {}", id, msg),
                SendError => "SendError".to_owned(),
                ParseIntError(err) => format!("{}", err),
                Utf8Error(err) => format!("{}", err),
            }
        )
    }
}

impl Decode<Error> for Error {
    type Err = Error;

    fn decode(buf: &[u8]) -> Result<Error, Error> {
        let (mut id, mut msg) = (0, String::new());

        for s in buf.split(|c| *c == b' ') {
            let parts: Vec<&[u8]> = s.splitn(2, |c| *c == b'=').collect();

            match *parts.get(0).unwrap() {
                b"id" => id = match u16::decode(parts.get(1).unwrap()) {
                    Ok(id) => id,
                    Err(err) => return Err(err.into()),
                },
                b"msg" => msg = match String::decode(parts.get(1).unwrap()) {
                    Ok(msg) => msg,
                    Err(err) => return Err(err.into()),
                },
                _ => (),
            }
        }

        Ok(Error::TS3{id, msg})
    }
}
