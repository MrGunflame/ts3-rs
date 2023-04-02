// Required for ts3_derive macro.
#[allow(unused_imports)]
use crate as ts3;

use crate::client::Client;
use crate::shared::list::Comma;
use crate::shared::{ChannelGroupId, ChannelId, ClientDatabaseId, ClientId, List, ServerGroupId};
use crate::{Decode, DecodeError, Error, ErrorKind};
use async_trait::async_trait;
use tokio::task::spawn;

impl Client {
    // Check buf for an event key. If one is found, a new task is spawned, the event
    // is dispatched to the associated handler and true is returned. If buf does not
    // contain event data, false is returned.
    pub(crate) fn dispatch_event(&self, buf: &[u8]) -> bool {
        let c = self.clone();
        let handler = c.inner.read().unwrap().handler.clone();

        // Split of the first argument (separated by ' '). It contains the event name.
        // The rest of the buffer contains the event data.
        let (event_name, rest): (&[u8], &[u8]);
        {
            let vec: Vec<&[u8]> = buf.splitn(2, |c| *c == b' ').collect();
            event_name = vec[0];
            rest = vec[1];
        }

        // buf contains the event data which will be moved to the event task.
        let buf = rest.to_owned();

        match event_name {
            b"notifycliententerview" => {
                let event = match ClientEnterView::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.cliententerview(c, event).await });
            }
            b"notifyclientleftview" => {
                let event = match ClientLeftView::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.clientleftview(c, event).await });
            }
            b"notifyserveredited" => {
                let event = match ServerEdited::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.serveredited(c, event).await });
            }
            b"notifychanneldescriptionchanged" => {
                let event = match ChannelDescriptionChanged::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.channeldescriptionchanged(c, event).await });
            }
            b"notifychannelpasswordchanged" => {
                let event = match ChannelPasswordChanged::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.channelpasswordchanged(c, event).await });
            }
            b"notifychannelmoved" => {
                let event = match ChannelMoved::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.channelmoved(c, event).await });
            }
            b"notifychanneledited" => {
                let event = match ChannelEdited::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.channeledited(c, event).await });
            }
            b"notifychannelcreated" => {
                let event = match ChannelCreated::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.channelcreated(c, event).await });
            }
            b"notifychanneldeleted" => {
                let event = match ChannelDeleted::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.channeldeleted(c, event).await });
            }
            b"notifyclientmoved" => {
                let event = match ClientMoved::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.clientmoved(c, event).await });
            }
            b"notifytextmessage" => {
                let event = match TextMessage::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.textmessage(c, event).await });
            }
            b"notifytokenused" => {
                let event = match TokenUsed::decode(&buf) {
                    Ok(event) => event,
                    Err(err) => {
                        handler.error(c, err);
                        return true;
                    }
                };

                spawn(async move { handler.tokenused(c, event).await });
            }
            _ => return false,
        }

        true
    }
}

/// All events sent by the server will be dispatched to their appropriate trait method.
/// In order to receive events you must subscribe to the events you want to receive using servernotifyregister.
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn cliententerview(&self, _client: Client, _event: ClientEnterView) {}
    async fn clientleftview(&self, _client: Client, _event: ClientLeftView) {}
    async fn serveredited(&self, _client: Client, _event: ServerEdited) {}
    async fn channeldescriptionchanged(&self, _client: Client, _event: ChannelDescriptionChanged) {}
    async fn channelpasswordchanged(&self, _client: Client, _event: ChannelPasswordChanged) {}
    async fn channelmoved(&self, _client: Client, _event: ChannelMoved) {}
    async fn channeledited(&self, _client: Client, _event: ChannelEdited) {}
    async fn channelcreated(&self, _client: Client, _event: ChannelCreated) {}
    async fn channeldeleted(&self, _client: Client, _event: ChannelDeleted) {}
    async fn clientmoved(&self, _client: Client, _event: ClientMoved) {}
    async fn textmessage(&self, _client: Client, _event: TextMessage) {}
    async fn tokenused(&self, _client: Client, _event: TokenUsed) {}

    fn error(&self, _client: Client, error: Error) {
        println!("connection error: {}", error);
    }
}

/// Defines a reason why an event happened. Used in multiple event types.
#[derive(Debug)]
pub enum ReasonId {
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

impl Decode for ReasonId {
    type Error = Error;

    fn decode(buf: &[u8]) -> Result<ReasonId, Self::Error> {
        match u8::decode(buf)? {
            0 => Ok(Self::SwitchChannel),
            1 => Ok(Self::Moved),
            2 => Ok(Self::Timeout),
            3 => Ok(Self::ChannelKick),
            4 => Ok(Self::ServerKick),
            5 => Ok(Self::Ban),
            6 => Ok(Self::ServerLeave),
            7 => Ok(Self::Edited),
            8 => Ok(Self::ServerShutdown),
            b => Err(Error(ErrorKind::Decode(DecodeError::InvalidReasonId(b)))),
        }
    }
}

impl Default for ReasonId {
    fn default() -> ReasonId {
        ReasonId::SwitchChannel
    }
}

/// Data for a `cliententerview` event.
#[derive(Debug, Decode, Default)]
pub struct ClientEnterView {
    pub cfid: ChannelId,
    pub ctid: ChannelId,
    pub reasonid: ReasonId,
    pub clid: ClientId,
    pub client_unique_identifier: String,
    pub client_nickname: String,
    pub client_input_muted: bool,
    pub client_output_muted: bool,
    pub client_outputonly_muted: bool,
    pub client_input_hardware: u64,
    pub client_output_hardware: u64,
    // client_meta_data: (),
    pub client_is_recording: bool,
    pub client_database_id: ClientDatabaseId,
    pub client_channel_group_id: ChannelGroupId,
    pub client_servergroups: List<ServerGroupId, Comma>,
    pub client_away: bool,
    pub client_away_message: String,
    pub client_type: u8,
    // client_flag_avatar: (),
    pub client_talk_power: u64,
    pub client_talk_request: bool,
    pub client_talk_request_msg: String,
    pub client_description: String,
    pub client_is_talker: bool,
    pub client_nickname_phoentic: String,
    pub client_needed_serverquey_view_power: u64,
    pub client_icon_id: u64,
    pub client_country: String,
    pub client_channel_group_inherited_channel_id: u64,
    pub client_badges: String,
}

/// Data for a `clientleftview` event.
#[derive(Debug, Decode, Default)]
pub struct ClientLeftView {
    pub cfid: ChannelId,
    pub ctid: ChannelId,
    pub reasonid: ReasonId,
    pub invokerid: ClientId,
    pub invokername: String,
    pub invokeruid: String,
    pub reasonmsg: String,
    pub bantime: u64,
    pub clid: ClientId,
}

/// Data for a `serveredited` event.
#[derive(Debug, Decode, Default)]
pub struct ServerEdited {
    pub reasonid: ReasonId,
    pub invokerid: ClientId,
    pub invokername: String,
    pub invokeruid: String,
    pub virtualserver_name: String,
    pub virtualserver_codec_encryption_mode: String,
    pub virtualserver_default_server_group: ServerGroupId,
    pub virtualserver_default_channel_group: ChannelGroupId,
    pub virtualserver_hostbanner_url: String,
    pub virtualserver_hostbanner_gfx_url: String,
    pub virtualserver_hostbanner_gfx_interval: u64,
    pub virtualserver_priority_speaker_dimm_modificator: String,
    pub virtualserver_hostbutton_tooltip: String,
    pub virtualserver_hostbutton_url: String,
    pub virtualserver_hostbutton_gfx_url: String,
    pub virtualserver_name_phoentic: String,
    pub virtualserver_icon_id: u64,
    pub virtualserver_hostbanner_mode: String,
    pub virtualserver_channel_temp_delete_delay_default: u64,
}

/// Data for a `channeldescriptionchanged` event.
#[derive(Debug, Decode, Default)]
pub struct ChannelDescriptionChanged {
    pub cid: ChannelId,
}

/// Data for a `channelpasswordchanged` event.
#[derive(Debug, Decode, Default)]
pub struct ChannelPasswordChanged {
    pub cid: ChannelId,
}

/// Data for a `channelmoved` event.
#[derive(Debug, Decode, Default)]
pub struct ChannelMoved {
    pub cid: ChannelId,
    pub cpid: ChannelId,
    pub order: u64,
    pub reasonid: ReasonId,
    pub invokerid: ClientId,
    pub invokername: String,
    pub invokeruid: String,
}

/// Data for a `channeledited` event. The fields `cid`, `reasonid`,
/// `invokerid`, `invokername` and `invokeruid` are always included.
/// All fields prefixed channel_... are only included if the value of
/// the channel was changed.
#[derive(Debug, Decode, Default)]
pub struct ChannelEdited {
    pub cid: ChannelId,
    pub reasonid: ReasonId,
    pub invokerid: ClientId,
    pub invokername: String,
    pub invokeruid: String,
    pub channel_name: String,
    pub channel_topic: String,
    // 4 for Opus Voice, 5 for Opus Music
    pub channel_codec: u8,
    pub channel_codec_quality: u8,
    pub channel_maxclients: u16,
    pub channel_maxfamilyclients: u16,
    pub channel_order: u64,
    pub channel_flag_permanent: bool,
    pub channel_flag_semi_permanent: bool,
    pub channel_flag_default: bool,
    pub channel_flag_password: String,
    pub channel_codec_latency_factor: u64,
    pub channel_codec_is_unencrypted: bool,
    pub channel_delete_delay: u32,
    pub channel_flag_maxclients_unlimited: bool,
    pub channel_flag_maxfamilyclients_unlimited: bool,
    pub channel_flag_maxfamilyclients_inherited: bool,
    pub channel_needed_talk_power: u32,
    pub channel_name_phonetic: String,
    pub channel_icon_id: u64,
}

/// Data for a `channelcreated` event.
#[derive(Debug, Decode, Default)]
pub struct ChannelCreated {
    pub cid: ChannelId,
    pub cpid: ChannelId,
    pub channel_name: String,
    pub channel_topic: String,
    // 4 for Opus Voice, 5 for Opus Music
    pub channel_codec: u8,
    pub channel_codec_quality: u8,
    pub channel_maxclients: u16,
    pub channel_maxfamilyclients: u16,
    pub channel_order: u64,
    pub channel_flag_permanent: bool,
    pub channel_flag_semi_permanent: bool,
    pub channel_flag_default: bool,
    pub channel_flag_password: bool,
    pub channel_codec_latency_factor: u64,
    pub channel_codec_is_unencrypted: bool,
    pub channel_delete_delay: u32,
    pub channel_flag_maxclients_unlimited: bool,
    pub channel_flag_maxfamilyclients_unlimited: bool,
    pub channel_flag_maxfamilyclients_inherited: bool,
    pub channel_needed_talk_power: u32,
    pub channel_name_phonetic: String,
    pub channel_icon_id: u64,
    pub invokerid: ClientId,
    pub invokername: String,
    pub invokeruid: String,
}

/// Data for a `channeldeleted` event.
#[derive(Debug, Decode, Default)]
pub struct ChannelDeleted {
    /// 0 if deleted by the server after exceeding the channel_delete_delay.
    pub invokerid: ClientId,
    /// "Server" if deleted by the server after exceeding the channel_delete_delay.
    pub invokername: String,
    /// Empty if deleted by the server after exceeding the channel_delete_delay.
    pub invokeruid: String,
    pub cid: ChannelId,
}

/// Data for a `clientmoved` event.
#[derive(Debug, Decode, Default)]
pub struct ClientMoved {
    pub ctid: ChannelId,
    pub reasonid: ReasonId,
    pub invokerid: ClientId,
    pub invokername: String,
    pub invokeruid: String,
    pub clid: ChannelId,
}

/// Data for a `textmessage` event.
#[derive(Debug, Decode, Default)]
pub struct TextMessage {
    pub targetmode: u64,
    pub msg: String,
    pub target: ClientId,
    pub invokerid: ClientId,
    pub invokername: String,
    pub invokeruid: String,
}

/// Data for a `tokenused` event.
#[derive(Debug, Decode, Default)]
pub struct TokenUsed {
    pub clid: ClientId,
    pub cldbid: ClientDatabaseId,
    pub cluid: String,
    pub token: String,
    pub tokencustomset: String,
    /// GroupID assigned by the token.
    pub token1: u64,
    /// ChannelID for the token, 0 if Server Group.
    pub token2: u64,
}

// Empty default impl for EventHandler
// Used internally as a default handler
pub(crate) struct Handler;

impl EventHandler for Handler {}
