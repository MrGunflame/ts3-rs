use std::marker::PhantomData;

use crate::{Decode, Encode};

/// A list of elements separated by a [`Separator`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct List<T, S>
where
    S: Separator,
{
    vec: Vec<T>,
    _marker: PhantomData<fn() -> S>,
}

impl<T, S> List<T, S>
where
    S: Separator,
{
    /// Creates a new `List` from a [`Vec`].
    #[inline]
    pub fn new(vec: Vec<T>) -> Self {
        Self {
            vec,
            _marker: PhantomData,
        }
    }

    /// Consumes this `List` and returns the wrapped [`Vec`].
    #[inline]
    pub fn into_inner(self) -> Vec<T> {
        self.vec
    }
}

impl<T, S> Encode for List<T, S>
where
    T: Encode,
    S: Separator,
{
    fn encode(&self, buf: &mut String) {
        if let Some(elem) = self.vec.get(0) {
            elem.encode(buf);
        }

        for elem in self.vec.iter().skip(1) {
            buf.push_str(S::PATTERN);
            elem.encode(buf);
        }
    }
}

impl<T, S> Decode for List<T, S>
where
    T: Decode,
    S: Separator,
{
    type Error = <T as Decode>::Error;

    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        let mut vec = Vec::new();

        for b in bytes_split(buf, S::PATTERN.as_bytes()) {
            vec.push(T::decode(b)?);
        }

        Ok(Self {
            vec,
            _marker: PhantomData,
        })
    }
}

/// A pattern used to separate elements in a [`List`].
pub trait Separator {
    /// The pattern used to separate the elements.
    const PATTERN: &'static str;
}

/// The pipe (`|`) separator.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pipe;

impl Separator for Pipe {
    const PATTERN: &'static str = "|";
}

/// The comma (`,`) separator.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Comma;

impl Separator for Comma {
    const PATTERN: &'static str = ",";
}

fn bytes_split<'a>(mut buf: &'a [u8], pat: &[u8]) -> Vec<&'a [u8]> {
    let mut cursor = 0;

    let mut segs = Vec::new();
    while buf.len() - cursor >= pat.len() {
        // Peek current position
        let slice = &buf[cursor..cursor + pat.len()];

        if slice == pat {
            segs.push(&buf[0..cursor]);

            // End of buffer
            if buf.len() <= pat.len() {
                return segs;
            }

            buf = &buf[cursor + pat.len()..];
            cursor = 0;
        } else {
            cursor += 1;
        }
    }

    // Remainder
    segs.push(buf);

    segs
}

#[cfg(test)]
mod tests {
    use crate::shared::list::bytes_split;

    #[test]
    fn test_bytes_split() {
        assert_eq!(bytes_split(b"a|b|c", b"|"), [b"a", b"b", b"c"]);
        assert_eq!(bytes_split(b"abc", b"|"), [b"abc"]);
        assert_eq!(
            bytes_split(b"a|bc", b"|"),
            [b"a".as_slice(), b"bc".as_slice()]
        );
        assert_eq!(
            bytes_split(b"a|bc|", b"|"),
            [b"a".as_slice(), b"bc".as_slice(), b"".as_slice()]
        );
        assert_eq!(bytes_split(b"ABCabcABC", b"abc"), [b"ABC", b"ABC"]);

        assert_eq!(
            bytes_split(b"00abcd0e0f00g000", b"0"),
            [
                b"".as_slice(),
                b"".as_slice(),
                b"abcd".as_slice(),
                b"e".as_slice(),
                b"f".as_slice(),
                b"".as_slice(),
                b"g".as_slice(),
                b"".as_slice(),
                b"".as_slice()
            ]
        );
    }
}
