//! Response types returned by client requests

use crate::client::APIKeyScope;
use crate::Decode;

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
    pub scope: APIKeyScope,
    pub time_left: u64,
}
