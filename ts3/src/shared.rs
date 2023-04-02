//! Types shared between requests/responses.

pub mod list;

use crate::{Decode, DecodeError, Encode, Error, ErrorKind};

pub use crate::types::{
    ApiKeyId, ChannelGroupId, ChannelId, ClientDatabaseId, ClientId, ServerGroupId, ServerId,
};

pub use list::List;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ApiKeyScope {
    Manage,
    Write,
    Read,
}

impl ApiKeyScope {
    const MANAGE: &str = "manage";
    const WRITE: &str = "write";
    const READ: &str = "read";
}

impl Default for ApiKeyScope {
    fn default() -> Self {
        Self::Manage
    }
}

impl Encode for ApiKeyScope {
    fn encode(&self, buf: &mut String) {
        match self {
            Self::Manage => *buf += Self::MANAGE,
            Self::Write => *buf += Self::WRITE,
            Self::Read => *buf += Self::READ,
        }
    }
}

impl Decode for ApiKeyScope {
    type Error = Error;

    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        let s = String::decode(buf)?;

        match s.as_str() {
            Self::MANAGE => Ok(Self::Manage),
            Self::WRITE => Ok(Self::Write),
            Self::READ => Ok(Self::Read),
            _ => Err(Error(ErrorKind::Decode(DecodeError::InvalidApiKeyScope(s)))),
        }
    }
}
