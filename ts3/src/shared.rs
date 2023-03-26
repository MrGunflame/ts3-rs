//! Types shared between requests/responses.

use crate::{Decode, DecodeError, Error, ErrorKind};

pub use crate::types::{
    ChannelGroupId, ChannelId, ClientDatabaseId, ClientId, ServerGroupId, ServerId,
};

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

    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Manage => Self::MANAGE,
            Self::Write => Self::WRITE,
            Self::Read => Self::READ,
        }
    }
}

impl Default for ApiKeyScope {
    fn default() -> Self {
        Self::Manage
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
