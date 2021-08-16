// Required for ts3_derive macro.
#[allow(unused_imports)]
use crate as ts3;

use crate::client::Client;
use crate::{BoxError, Decode, ParseError};
use async_trait::async_trait;
use std::str::FromStr;
use tokio::task;

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
                task::spawn(async move {
                    handler
                        .cliententerview(c, ClientEnterView::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifyclientleftview" => {
                task::spawn(async move {
                    handler
                        .clientleftview(c, ClientLeftView::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifyserveredited" => {
                task::spawn(async move {
                    handler
                        .serveredited(c, ServerEdited::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifychanneldescriptionchanged" => {
                task::spawn(async move {
                    handler
                        .channeldescriptionchanged(
                            c,
                            ChannelDescriptionChanged::decode(&buf).unwrap(),
                        )
                        .await;
                });
                true
            }
            b"notifychannelpasswordchanged" => {
                task::spawn(async move {
                    handler
                        .channelpasswordchanged(c, ChannelPasswordChanged::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifychannelmoved" => {
                task::spawn(async move {
                    handler
                        .channelmoved(c, ChannelMoved::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifychanneledited" => {
                task::spawn(async move {
                    handler
                        .channeledited(c, ChannelEdited::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifychannelcreated" => {
                task::spawn(async move {
                    handler
                        .channelcreated(c, ChannelCreated::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifychanneldeleted" => {
                task::spawn(async move {
                    handler
                        .channeldeleted(c, ChannelDeleted::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifyclientmoved" => {
                task::spawn(async move {
                    handler
                        .clientmoved(c, ClientMoved::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifytextmessage" => {
                task::spawn(async move {
                    handler
                        .textmessage(c, TextMessage::decode(&buf).unwrap())
                        .await;
                });
                true
            }
            b"notifytokenused" => {
                task::spawn(async move {
                    handler.tokenused(c, TokenUsed::decode(&buf).unwrap()).await;
                });
                true
            }
            _ => false,
        }
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
}

/// Defines a reason why an event happened. Used in multiple event types.
#[derive(Debug)]
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

impl Decode<ReasonID> for ReasonID {
    fn decode(buf: &[u8]) -> Result<ReasonID, BoxError> {
        use ReasonID::*;
        Ok(match u8::decode(buf)? {
            0 => SwitchChannel,
            1 => Moved,
            2 => Timeout,
            3 => ChannelKick,
            4 => ServerKick,
            5 => Ban,
            6 => ServerLeave,
            7 => Edited,
            8 => ServerShutdown,
            n => panic!("Unexpected Reasonid {}", n),
        })
    }
}

impl Default for ReasonID {
    fn default() -> ReasonID {
        ReasonID::SwitchChannel
    }
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

/// Data for a `cliententerview` event.
#[derive(Debug, Decode, Default)]
pub struct ClientEnterView {
    pub cfid: u64,
    pub ctid: u64,
    pub reasonid: ReasonID,
    pub clid: u64,
    pub client_unique_identifier: String,
    pub client_nickname: String,
    pub client_input_muted: bool,
    pub client_output_muted: bool,
    pub client_outputonly_muted: bool,
    pub client_input_hardware: u64,
    pub client_output_hardwarer: u64,
    // client_meta_data: (),
    pub client_is_recording: bool,
    pub client_database_id: u64,
    pub client_channel_group_id: u64,
    pub client_servergroups: Vec<u64>,
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
    pub cfid: usize,
    pub ctid: usize,
    pub reasonid: ReasonID,
    pub invokerid: usize,
    pub invokername: String,
    pub invokeruid: String,
    pub reasonmsg: String,
    pub bantime: usize,
    pub clid: usize,
}

/// Data for a `serveredited` event.
#[derive(Debug, Decode, Default)]
pub struct ServerEdited {
    pub reasonid: ReasonID,
    pub invokerid: u64,
    pub invokername: String,
    pub invokeruid: String,
    pub virtualserver_name: String,
    pub virtualserver_codec_encryption_mode: String,
    pub virtualserver_default_server_group: u64,
    pub virtualserver_default_channel_group: u64,
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
    pub cid: u64,
}

/// Data for a `channelpasswordchanged` event.
#[derive(Debug, Decode, Default)]
pub struct ChannelPasswordChanged {
    pub cid: u64,
}

/// Data for a `channelmoved` event.
#[derive(Debug, Decode, Default)]
pub struct ChannelMoved {
    pub cid: u64,
    pub cpid: u64,
    pub order: u64,
    pub reasonid: ReasonID,
    pub invokerid: u64,
    pub invokername: String,
    pub invokeruid: String,
}

/// Data for a `channeledited` event. The fields `cid`, `reasonid`,
/// `invokerid`, `invokername` and `invokeruid` are always included.
/// All fields prefixed channel_... are only included if the value of
/// the channel was changed.
#[derive(Debug, Decode, Default)]
pub struct ChannelEdited {
    pub cid: u64,
    pub reasonid: u64,
    pub invokerid: u64,
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
    pub cid: u64,
    pub cpid: u64,
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
    pub invokerid: u64,
    pub invokername: String,
    pub invokeruid: String,
}

/// Data for a `channeldeleted` event.
#[derive(Debug, Decode, Default)]
pub struct ChannelDeleted {
    /// 0 if deleted by the server after exceeding the channel_delete_delay.
    pub invokerid: u64,
    /// "Server" if deleted by the server after exceeding the channel_delete_delay.
    pub invokername: String,
    /// Empty if deleted by the server after exceeding the channel_delete_delay.
    pub invokeruid: String,
    pub cid: u64,
}

/// Data for a `clientmoved` event.
#[derive(Debug, Decode, Default)]
pub struct ClientMoved {
    pub ctid: u64,
    pub reasonid: ReasonID,
    pub invokerid: u64,
    pub invokername: String,
    pub invokeruid: String,
    pub clid: u64,
}

/// Data for a `textmessage` event.
#[derive(Debug, Decode, Default)]
pub struct TextMessage {
    pub targetmode: usize,
    pub msg: String,
    pub target: usize,
    pub invokerid: usize,
    pub invokername: String,
    pub invokeruid: String,
}

/// Data for a `tokenused` event.
#[derive(Debug, Decode, Default)]
pub struct TokenUsed {
    pub clid: u64,
    pub cldbid: u64,
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
