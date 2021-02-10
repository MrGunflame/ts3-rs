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

pub use client::{Client, Error, RawResp};

use std::str::FromStr;

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
