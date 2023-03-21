// Required for ts3_derive macro.
#[allow(unused_imports)]
use crate as ts3;

pub use async_trait::async_trait;

use crate::{
    event::{EventHandler, Handler},
    response::{ApiKey, Version},
    shared::ApiKeyScope,
    ChannelId, ClientDatabaseId, ClientId, CommandBuilder, Decode, Encode, Error, ErrorKind,
    ServerGroupId, ServerId,
};
use bytes::Bytes;
use std::{
    collections::HashMap,
    convert::From,
    fmt::{self, Display, Formatter, Write},
    result,
    str::from_utf8,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, ToSocketAddrs},
    sync::{mpsc, oneshot},
    task::spawn,
    time::sleep,
};

pub type Result<T> = result::Result<T, Error>;

impl Error {
    fn ok(&self) -> bool {
        use ErrorKind::*;

        match &self.0 {
            TS3 { id, msg: _ } => *id == 0,
            _ => false,
        }
    }
}

// Read a error from a raw server response
impl From<RawResp> for Error {
    fn from(mut resp: RawResp) -> Error {
        Error(ErrorKind::TS3 {
            id: resp.items[0]
                .remove("id")
                .unwrap()
                .unwrap()
                .parse()
                .unwrap(),
            msg: resp.items[0].remove("msg").unwrap().unwrap(),
        })
    }
}

struct Cmd {
    bytes: Bytes,
    resp: oneshot::Sender<Result<Vec<u8>>>,
}

pub(crate) struct ClientInner {
    pub(crate) handler: Arc<dyn EventHandler>,
}

impl ClientInner {
    fn new() -> ClientInner {
        ClientInner {
            handler: Arc::new(Handler),
        }
    }
}

/// A Client used to send commands to the serverquery interface.
#[derive(Clone)]
pub struct Client {
    tx: mpsc::Sender<Cmd>,
    pub(crate) inner: Arc<RwLock<ClientInner>>,
}

impl Client {
    /// Create a new connection
    pub async fn new<A: ToSocketAddrs>(addr: A) -> Result<Client> {
        let (tx, mut rx) = mpsc::channel::<Cmd>(32);

        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| Error(e.into()))?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Read initial welcome message
        {
            let mut buf = Vec::new();
            let _ = reader.read_until(b'\r', &mut buf).await;
            buf.clear();
            let _ = reader.read_until(b'\r', &mut buf).await;
        }

        // read_tx and read_rx are used to communicate between the read and the write
        // thread
        let (read_tx, mut read_rx) = mpsc::channel(32);

        // Create a new inner client
        let client = Client {
            tx: tx,
            // handler: Arc::new(RwLock::new()),
            inner: Arc::new(RwLock::new(ClientInner::new())),
        };

        // Read task
        let client2 = client.clone();
        spawn(async move {
            loop {
                let client = client2.clone();

                // Read from the buffer until a '\r' indicating the end of a line
                let mut buf = Vec::new();
                if let Err(err) = reader.read_until(b'\r', &mut buf).await {
                    println!("{}", err);
                    continue;
                }

                // Remove the last two bytes '\n' and '\r'
                buf.truncate(buf.len() - 2);

                // If the received data is an event dispatch it to the correct handler and wait for
                // the next line.
                if client.dispatch_event(&buf) {
                    continue;
                }

                // Query commands return 2 lines, the first being the response data while the sencond
                // contains the error code. Other commands only return an error.
                match buf.starts_with(b"error") {
                    true => {
                        let _ = read_tx
                            .send((Vec::new(), Error::decode(&buf).unwrap()))
                            .await;
                    }
                    false => {
                        // Clone the current buffer, which contains the response data
                        let resp = buf.clone();

                        // Read next line for the error
                        buf.clear();
                        if let Err(err) = reader.read_until(b'\r', &mut buf).await {
                            eprintln!("{}", err);
                            continue;
                        }

                        let _ = read_tx.send((resp, Error::decode(&buf).unwrap())).await;
                    }
                }
            }
        });

        // Write Task
        spawn(async move {
            while let Some(cmd) = rx.recv().await {
                // Write the command string
                if let Err(err) = writer.write(&cmd.bytes).await {
                    let _ = cmd.resp.send(Err(Error(err.into())));
                    continue;
                }

                // Write a '\n' to send the command
                if let Err(err) = writer.write(&[b'\n']).await {
                    let _ = cmd.resp.send(Err(Error(err.into())));
                    continue;
                }

                // Wait for the response from the reader task
                let (resp, err) = read_rx.recv().await.unwrap();

                // Write the response to the channel sent with the request. resp is None when
                // an error occured.
                let _ = cmd.resp.send(match err.ok() {
                    true => Ok(resp),
                    false => Err(err),
                });
            }
        });

        // Keepalive loop
        let tx2 = client.tx.clone();
        spawn(async move {
            loop {
                let tx = tx2.clone();
                sleep(Duration::from_secs(60)).await;
                {
                    let (resp_tx, _) = oneshot::channel();
                    if let Err(_) = tx
                        .send(Cmd {
                            bytes: Bytes::from_static("version".as_bytes()),
                            resp: resp_tx,
                        })
                        .await
                    {}
                }
            }
        });

        Ok(client)
    }

    pub fn set_event_handler<H: EventHandler + 'static>(&self, handler: H) {
        let mut data = self.inner.write().unwrap();
        data.handler = Arc::new(handler);
    }

    /// Send a raw command directly to the server. The response will be directly decoded
    /// into the type `T`. To get a HashMap like response, use the `RawResp` struct.
    pub async fn send<T: Decode>(&self, cmd: String) -> Result<T> {
        let tx = self.tx.clone();

        // Create a new channel for receiving the response
        let (resp_tx, resp_rx) = oneshot::channel();

        match tx
            .send(Cmd {
                bytes: Bytes::from(cmd.into_bytes()),
                resp: resp_tx,
            })
            .await
        {
            Ok(_) => {
                let resp = resp_rx.await;
                Ok(T::decode(&resp.unwrap().unwrap()).unwrap())
            }
            Err(_) => Err(Error(ErrorKind::SendError)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ServerNotifyRegister {
    Server,
    Channel(ChannelId),
    TextServer,
    TextChannel,
    TextPrivate,
}

impl Display for ServerNotifyRegister {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use ServerNotifyRegister::*;
        write!(
            f,
            "{}",
            match self {
                Server => "server".to_owned(),
                Channel(cid) => format!("channel id={}", cid),
                TextServer => "textserver".to_owned(),
                TextChannel => "textchannel".to_owned(),
                TextPrivate => "textprivate".to_owned(),
            }
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextMessageTarget {
    Client(ClientId),
    Channel,
    Server,
}

impl Encode for TextMessageTarget {
    fn encode(&self, writer: &mut String) {
        use TextMessageTarget::*;
        write!(
            writer,
            "{}",
            match self {
                Client(clid) => format!("1 target={}", clid),
                Channel => "2".to_owned(),
                Server => "3".to_owned(),
            }
        )
        .unwrap();
    }
}

// TS3 Commands go here
impl Client {
    /// Creates a new apikey using the specified scope, for the invoking user. The default
    /// lifetime of a token is 14 days, a zero lifetime means no expiration. It is possible
    ///  to create apikeys for other users using `b_virtualserver_apikey_manage.`
    pub async fn apikeyadd(
        &self,
        scope: ApiKeyScope,
        lifetime: Option<u64>,
        cldbid: Option<u64>,
    ) -> Result<ApiKey> {
        self.send(format!(
            "apikeyadd scope={} {} {}",
            scope.as_str(),
            match lifetime {
                None => "".to_owned(),
                Some(lifetime) => format!("lifetime={}", lifetime),
            },
            match cldbid {
                None => "".to_owned(),
                Some(cldbid) => format!("cldbid={}", cldbid),
            }
        ))
        .await
    }

    /// Delete an apikey. Any apikey owned by the current user can always be deleted. Deleting
    /// apikeys from another user requires `b_virtualserver_apikey_manage`.
    pub async fn apikeydel(&self, id: u64) -> Result<()> {
        self.send(format!("apikeydel id={}", id)).await
    }

    /// Lists all apikeys owned by the user, or of all users using `cldbid`=`(0, true).` Usage
    /// of `cldbid`=... requires `b_virtualserver_apikey_manage`.
    pub async fn apikeylist(
        &self,
        cldbid: Option<(u64, bool)>,
        start: Option<u64>,
        duration: Option<u64>,
        count: bool,
    ) -> Result<Vec<ApiKey>> {
        self.send(format!(
            "apikeylist {} {} {} {}",
            match cldbid {
                None => "".to_owned(),
                Some((cldbid, all)) => format!(
                    "cldbid={}",
                    match all {
                        true => "*".to_owned(),
                        false => cldbid.to_string(),
                    }
                ),
            },
            match start {
                None => "".to_owned(),
                Some(start) => format!("start={}", start),
            },
            match duration {
                None => "".to_owned(),
                Some(duration) => format!("duration={}", duration),
            },
            match count {
                true => "-count",
                false => "",
            }
        ))
        .await
    }

    /// Add a new ban rule on the selected virtual server. One of `ip`, `name`, `uid`
    /// and `mytsid` must not be `None`.
    pub async fn banadd(
        &self,
        ip: Option<&str>,
        name: Option<&str>,
        uid: Option<&str>,
        mytsid: Option<&str>,
        time: Option<u64>,
        banreason: Option<&str>,
        lastnickname: Option<&str>,
    ) -> Result<()> {
        self.send({
            CommandBuilder::new("banadd")
                .arg_opt("ip", ip)
                .arg_opt("name", name)
                .arg_opt("uid", uid)
                .arg_opt("mytsid", mytsid)
                .arg_opt("time", time)
                .arg_opt("banreason", banreason)
                .arg_opt("lastnickname", lastnickname)
                .into_inner()
        })
        .await
    }

    /// Sends a text message to all clients on all virtual servers in the TeamSpeak 3
    /// Server instance.
    pub async fn gm(&self, msg: &str) -> Result<()> {
        self.send(format!("gm={}", msg)).await
    }

    /// Authenticate with the given data.
    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        self.send(
            CommandBuilder::new("login")
                .arg("client_login_name", username)
                .arg("client_login_password", password)
                .into_inner(),
        )
        .await?;
        Ok(())
    }

    /// Deselects the active virtual server and logs out from the server instance.
    pub async fn logout(&self) -> Result<()> {
        self.send("logout".to_owned()).await?;
        Ok(())
    }

    /// Send a quit command, disconnecting the client and closing the TCP connection
    pub async fn quit(&self) -> Result<()> {
        self.send("quit".to_owned()).await?;
        Ok(())
    }

    pub async fn sendtextmessage(&self, target: TextMessageTarget, msg: &str) -> Result<()> {
        self.send(
            CommandBuilder::new("sendtextmessage")
                .arg("targetmode", target)
                .arg("msg", msg)
                .into_inner(),
        )
        .await?;
        Ok(())
    }

    /// Adds one or more clients to the server group specified with sgid. Please note that a
    /// client cannot be added to default groups or template groups.
    pub async fn servergroupaddclient(
        &self,
        sgid: ServerGroupId,
        cldbid: ClientDatabaseId,
    ) -> Result<()> {
        self.send(
            CommandBuilder::new("servergroupaddclient")
                .arg("sgid", sgid)
                .arg("cldbid", cldbid)
                .into_inner(),
        )
        .await?;
        Ok(())
    }

    /// Removes one or more clients specified with cldbid from the server group specified with
    /// sgid.  
    pub async fn servergroupdelclient(
        &self,
        sgid: ServerGroupId,
        cldbid: ClientDatabaseId,
    ) -> Result<()> {
        self.send(
            CommandBuilder::new("servergroupdelclient")
                .arg("sgid", sgid)
                .arg("cldbid", cldbid)
                .into_inner(),
        )
        .await?;
        Ok(())
    }

    /// Registers for a specified category of events on a virtual server to receive
    /// notification messages. Depending on the notifications you've registered for,
    /// the server will send you a message on every event in the view of your
    /// ServerQuery client (e.g. clients joining your channel, incoming text
    /// messages, server configuration changes, etc). The event source is declared by
    /// the event parameter while id can be used to limit the notifications to a
    /// specific channel.  
    pub async fn servernotifyregister(&self, event: ServerNotifyRegister) -> Result<()> {
        self.send(format!("servernotifyregister event={}", event))
            .await?;
        Ok(())
    }

    /// Starts the virtual server specified with sid. Depending on your permissions,
    /// you're able to start either your own virtual server only or all virtual
    /// servers in the server instance.  
    pub async fn serverstart<T>(&self, sid: T) -> Result<()>
    where
        T: Into<ServerId>,
    {
        self.send(format!("serverstart sid={}", sid.into())).await
    }

    /// Stops the virtual server specified with sid. Depending on your permissions,
    /// you're able to stop either your own virtual server only or all virtual
    /// servers in the server instance. The reasonmsg parameter specifies a
    /// text message that is sent to the clients before the client disconnects.
    pub async fn serverstop<T>(&self, sid: T) -> Result<()>
    where
        T: Into<ServerId>,
    {
        self.send(format!("serverstop sid={}", sid.into())).await
    }

    /// Switch to the virtualserver (voice) with the given server id
    pub async fn use_sid<T>(&self, sid: T) -> Result<()>
    where
        T: Into<ServerId>,
    {
        self.send(format!("use sid={}", sid.into())).await?;
        Ok(())
    }

    /// Like `use_sid` but instead use_port uses the voice port to connect to the virtualserver
    pub async fn use_port(&self, port: u16) -> Result<()> {
        self.send(format!("use port={}", port)).await?;
        Ok(())
    }

    /// Returns information about the server version
    pub async fn version(&self) -> Result<Version> {
        self.send("version".to_owned()).await
    }

    /// Returns information about the query client connected
    pub async fn whoami(&self) -> Result<RawResp> {
        self.send("whoami".to_owned()).await
    }
}

/// RawResp contains all data returned from the server
/// When the items vector contains multiple entries, the server returned a list.
/// Otherwise only a single item will be in the vector
/// The HashMap contains all key-value pairs, but values are optional
#[derive(Clone, Debug)]
pub struct RawResp {
    pub items: Vec<HashMap<String, Option<String>>>,
}

impl From<&[u8]> for RawResp {
    fn from(buf: &[u8]) -> RawResp {
        let mut items = Vec::new();

        // Split all items lists into separate strings first
        // If the content is no list a single item is remained
        let res: Vec<&str> = from_utf8(&buf).unwrap().split("|").collect();
        for entry in res {
            let mut map = HashMap::new();

            // All key-value pairs are separated by ' '
            let res: Vec<&str> = entry.split(" ").collect();
            for item in res {
                // Each pair that contains a '=' has both a key and a value
                // A pair that has no '=' is only a key
                // Only split the first '=' as splitting multiple could split strings inside the value
                let parts: Vec<&str> = item.splitn(2, "=").collect();

                // Insert key and value when both exist
                // Otherwise None is inserted with the key
                map.insert(
                    parts.get(0).unwrap().to_string(),
                    match parts.len() {
                        n if n > 1 => Some(parts.get(1).unwrap().to_string()),
                        _ => None,
                    },
                );
            }

            items.push(map);
        }

        RawResp { items }
    }
}

impl Decode for RawResp {
    type Error = Error;

    fn decode(buf: &[u8]) -> result::Result<Self, Self::Error> {
        Ok(buf.into())
    }
}
