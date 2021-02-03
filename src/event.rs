use crate::client::{Client, RawResp};
use async_trait::async_trait;
use std::sync::Arc;

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
        "notifytextmessage" => handler.textmessage(c, resp),
        "notifytokenused" => handler.tokenused(c, resp),
        _ => unreachable!(),
    }
    .await;
}

/// All events sent by the server will be dispatched to their appropriate trait method.
/// In order to receive events you must subscribe to the events you want to receive using servernotifyregister.
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn cliententerview(&self, client: Client, event: RawResp) {}
    async fn clientleftview(&self, client: Client, event: RawResp) {}
    async fn serveredited(&self, client: Client, event: RawResp) {}
    async fn channeldescriptionchanged(&self, client: Client, event: RawResp) {}
    async fn channelpasswordchanged(&self, client: Client, event: RawResp) {}
    async fn channelmoved(&self, client: Client, event: RawResp) {}
    async fn channeledited(&self, client: Client, event: RawResp) {}
    async fn channelcreated(&self, client: Client, event: RawResp) {}
    async fn channeldeleted(&self, client: Client, event: RawResp) {}
    async fn clientmoved(&self, client: Client, event: RawResp) {}
    async fn textmessage(&self, client: Client, event: RawResp) {}
    async fn tokenused(&self, client: Client, event: RawResp) {}
}

// Empty default impl for EventHandler
// Used internally as a default handler
pub(crate) struct Handler;

impl EventHandler for Handler {}
