use crate::client::{Client, RawResp};
use crate::ParseError;
use async_trait::async_trait;
use std::convert::From;
use std::str::FromStr;

// Returns Some(event_name) when the response is an event message and None if it is none
pub(crate) fn is_event(resp: &RawResp) -> Option<String> {
    for name in vec![
        "notifycliententerview",
        "notifyclientleftview",
        "notifyserveredited",
        "notifychanneldescriptionchanged",
        "notifychannelpasswordchanged",
        "notifychannelmoved",
        "notifychanneledited",
        "notifychannelcreated",
        "notifychanneldeleted",
        "notifyclientmoved",
        "notifytextmessage",
        "notifytokenused",
    ] {
        if resp.items[0].contains_key(name) {
            return Some(name.to_owned());
        }
    }

    None
}

// Dispatch the event to the appropriate handler method. This function will stay alive as
// long as the handler method executes, so this should be moved to a separate task.
pub(crate) async fn dispatch_event(c: Client, resp: RawResp, event: &str) {
    let c2 = c.clone();
    let handler = c2.inner.read().unwrap().handler.clone();

    match event {
        "notifycliententerview" => handler.cliententerview(c, resp),
        "notifyclientleftview" => handler.clientleftview(c, resp),
        "notifyserveredited" => handler.serveredited(c, resp),
        "notifychanneldescriptionchanged" => handler.channeldescriptionchanged(c, resp),
        "notifychannelpasswordchanged" => handler.channelpasswordchanged(c, resp),
        "notifychannelmoved" => handler.channelmoved(c, resp),
        "notifychanneledited" => handler.channeledited(c, resp),
        "notifychannelcreated" => handler.channelcreated(c, resp),
        "notifychanneldeleted" => handler.channeldeleted(c, resp),
        "notifyclientmoved" => handler.clientmoved(c, resp),
        "notifytextmessage" => handler.textmessage(c, resp.into()),
        "notifytokenused" => handler.tokenused(c, resp),
        _ => unreachable!(),
    }
    .await;
}

/// All events sent by the server will be dispatched to their appropriate trait method.
/// In order to receive events you must subscribe to the events you want to receive using servernotifyregister.
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn cliententerview(&self, _client: Client, _event: RawResp) {}
    async fn clientleftview(&self, _client: Client, _event: RawResp) {}
    async fn serveredited(&self, _client: Client, _event: RawResp) {}
    async fn channeldescriptionchanged(&self, _client: Client, _event: RawResp) {}
    async fn channelpasswordchanged(&self, _client: Client, _event: RawResp) {}
    async fn channelmoved(&self, _client: Client, _event: RawResp) {}
    async fn channeledited(&self, _client: Client, _event: RawResp) {}
    async fn channelcreated(&self, _client: Client, _event: RawResp) {}
    async fn channeldeleted(&self, _client: Client, _event: RawResp) {}
    async fn clientmoved(&self, _client: Client, _event: RawResp) {}
    async fn textmessage(&self, _client: Client, _event: TextMessage) {}
    async fn tokenused(&self, _client: Client, _event: RawResp) {}
}

pub struct ClientLeftView {
    cfid: usize,
    ctid: usize,
    reasonid: ReasonID,
    invokerid: usize,
    invokername: String,
    invokeruid: String,
    reasonmsg: String,
    bantime: usize,
    clid: usize,
}

/// Defines a reason why an event happened. Used in multiple event types.
pub enum ReasonID {
    /// Switched channel themselves or joined server
    SwitchChannel = 0,
    // Moved by another client or channel
    Moved,
    // Left server because of timeout (disconnect)
    Timeout,
    // Kicked from channel
    ChannelKick,
    // Kicked from server
    ServerKick,
    // Banned from server
    Ban,
    // Left server themselves
    ServerLeave,
    // Edited channel or server
    Edited,
    // Left server due shutdown
    ServerShutdown,
}

impl FromStr for ReasonID {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<ReasonID, Self::Err> {
        match s {
            "0" => Ok(ReasonID::SwitchChannel),
            "1" => Ok(ReasonID::Moved),
            "3" => Ok(ReasonID::Timeout),
            "4" => Ok(ReasonID::ChannelKick),
            "5" => Ok(ReasonID::ServerKick),
            "6" => Ok(ReasonID::Ban),
            "8" => Ok(ReasonID::ServerLeave),
            "10" => Ok(ReasonID::Edited),
            "11" => Ok(ReasonID::ServerShutdown),
            _ => Err(ParseError::InvalidEnum),
        }
    }
}

/// Returned from a "textmessage" event
#[derive(Clone, Debug)]
pub struct TextMessage {
    pub targetmode: usize,
    pub msg: String,
    pub target: usize,
    pub invokerid: usize,
    pub invokername: String,
    pub invokeruid: String,
}

impl From<RawResp> for TextMessage {
    fn from(raw: RawResp) -> TextMessage {
        TextMessage {
            targetmode: get_field(&raw, "targetmod"),
            msg: get_field(&raw, "msg"),
            target: get_field(&raw, "target"),
            invokerid: get_field(&raw, "invokerid"),
            invokername: get_field(&raw, "invokername"),
            invokeruid: get_field(&raw, "invokeruid"),
        }
    }
}

// Empty default impl for EventHandler
// Used internally as a default handler
pub(crate) struct Handler;

impl EventHandler for Handler {}

fn get_field<T>(raw: &RawResp, name: &str) -> T
where
    T: FromStr + Default,
{
    match raw.items.get(0) {
        Some(val) => match val.get(name) {
            Some(val) => match val {
                Some(val) => match T::from_str(&val) {
                    Ok(val) => val,
                    Err(_) => T::default(),
                },
                None => T::default(),
            },
            None => T::default(),
        },
        None => T::default(),
    }
}
