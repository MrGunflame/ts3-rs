//! Response types returned by client requests

use std::collections::HashMap;
use std::ops::Deref;

use crate::shared::ApiKeyScope;
use crate::types::{ServerId, ClientId, ChannelId, ClientDatabaseId};
use crate::{Decode, DecodeError, Error, ErrorKind};

/// A raw response of at least one [`Entry`].
#[derive(Clone, Debug)]
pub struct Response {
    entries: Vec<Entry>,
}

impl Deref for Response {
    type Target = [Entry];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl Decode for Response {
    type Error = Error;

    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        let mut entries = Vec::new();

        for entry in buf.split(|b| *b == b'|') {
            let entry = Entry::decode(entry)?;
            entries.push(entry);
        }

        Ok(Self { entries })
    }
}

/// A single entry of key-value pairs.
#[derive(Clone, Debug)]
pub struct Entry {
    fields: HashMap<String, Option<String>>,
}

impl Entry {
    /// Returns `true` if the `Entry` contains the given `key`.
    #[inline]
    pub fn contains(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// Parses and returns the value of a given `key` as `T`.
    ///
    /// # Errors
    ///
    /// This function returns an [`Error`] if the requested `key` does not exist, contains no value
    /// or cannot be decoded into `T`.
    pub fn get<T>(&self, key: &str) -> Result<T, Error>
    where
        T: Decode,
        T::Error: Into<Error>,
    {
        let Some(value) = self.fields.get(key) else {
            return Err(Error(ErrorKind::NoField));
        };

        let Some(value) = value else {
            return Err(Error(ErrorKind::NoField));
        };

        T::decode(value.as_bytes()).map_err(|e| e.into())
    }
}

impl Decode for Entry {
    type Error = Error;

    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        let mut entry = HashMap::new();

        // KV pairs separated by ' '.
        for item in buf.split(|c| *c == b' ') {
            let mut parts = item.splitn(2, |c| *c == b'=');

            let Some(key) = parts.next() else {
                return Err(Error(DecodeError::UnexpectedEof.into()));
            };

            let key = match std::str::from_utf8(key) {
                Ok(key) => key.to_owned(),
                Err(err) => return Err(Error(err.into())),
            };

            let value = match parts.next() {
                Some(value) => {
                    let value = match std::str::from_utf8(value) {
                        Ok(value) => value,
                        Err(err) => return Err(Error(err.into())),
                    };

                    Some(value.to_owned())
                }
                None => None,
            };

            entry.insert(key, value);
        }

        Ok(Self { fields: entry })
    }
}

/// Data returned from the `version` command.
#[derive(Debug, Decode, Default)]
pub struct Version {
    pub version: String,
    pub build: u64,
    pub platform: String,
}

/// An API Key returned from [`Client.apikeyadd`].
#[derive(Debug, Decode, Default)]
pub struct ApiKey {
    pub apikey: String,
    pub id: u64,
    pub sid: u64,
    pub cldbid: u64,
    pub scope: ApiKeyScope,
    pub time_left: u64,
}

#[derive(Clone, Debug, Default, Decode)]
pub struct Whoami {
    pub virtualserver_status: VirtualServerStatus,
    pub virtualserver_unique_identifier: String,
    pub virtualserver_port: u16,
    pub virtualserver_id: ServerId,
    pub client_id: ClientId,
    pub client_channel_id: ChannelId,
    pub client_nickname: String,
    pub client_database_id: ClientDatabaseId,
    pub client_login_name: String,
    pub client_unique_identifier: String,
    pub client_origin_server_id: ServerId,
    _priv: (),
}

#[derive(Copy, Clone, Debug, Default)]
pub enum VirtualServerStatus {
    #[default]
    Unknown,
    Online,
    Offline,
}

impl Decode for VirtualServerStatus {
    type Error = Error;

    fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
        match buf {
            b"unknown" => Ok(Self::Unknown),
            b"online" => Ok(Self::Online),
            b"offline" => Ok(Self::Offline),
            _ => Err(Error(DecodeError::UnexpectedEof.into())),
        }
    }
}

