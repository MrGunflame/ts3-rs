// Required for ts3_derive macro.
#[allow(unused_imports)]
use crate as ts3;
use crate::request::{Request, RequestBuilder, ServerNotifyRegister, TextMessageTarget};
use crate::response::Response;
use crate::shared::list::Pipe;

pub use async_trait::async_trait;

use crate::shared::{ClientDatabaseId, List, ServerGroupId, ServerId};
use crate::{
    event::{EventHandler, Handler},
    response::{ApiKey, Version},
    shared::ApiKeyScope,
    Decode, Error, ErrorKind,
};
use bytes::Bytes;
use std::{
    convert::From,
    result,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::{
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
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<Client> {
        let (tx, mut rx) = mpsc::channel::<Cmd>(32);

        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| Error(e.into()))?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Read initial welcome message
        {
            let mut buf = Vec::new();
            reader
                .read_until(b'\r', &mut buf)
                .await
                .map_err(|e| Error(e.into()))?;
            buf.clear();
            reader
                .read_until(b'\r', &mut buf)
                .await
                .map_err(|e| Error(e.into()))?;
        }

        // read_tx and read_rx are used to communicate between the read and the write
        // thread
        let (read_tx, mut read_rx) = mpsc::channel(32);

        // Create a new inner client
        let client = Client {
            tx,
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
                    client.handle_error(Error(err.into()));
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
                    true => match Error::decode(&buf) {
                        Ok(err) => {
                            let _ = read_tx.send((Vec::new(), err)).await;
                        }
                        Err(err) => {
                            client.handle_error(err);
                        }
                    },
                    false => {
                        // Clone the current buffer, which contains the response data
                        let resp = buf.clone();

                        // Read next line for the error
                        buf.clear();
                        if let Err(err) = reader.read_until(b'\r', &mut buf).await {
                            client.handle_error(Error(err.into()));
                            continue;
                        }

                        match Error::decode(&buf) {
                            Ok(err) => {
                                let _ = read_tx.send((resp, err)).await;
                            }
                            Err(err) => {
                                client.handle_error(err);
                            }
                        }
                    }
                }
            }
        });

        // Write Task
        spawn(async move {
            while let Some(cmd) = rx.recv().await {
                // Write the command string
                if let Err(err) = writer.write_all(&cmd.bytes).await {
                    let _ = cmd.resp.send(Err(Error(err.into())));
                    continue;
                }

                // Write a '\n' to send the command
                if let Err(err) = writer.write_all(&[b'\n']).await {
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

    /// Sends a [`Request`] to the server.
    pub async fn send<T, R>(&self, request: R) -> Result<T>
    where
        T: Decode,
        T::Error: Into<Error>,
        R: Into<Request>,
    {
        self.send_inner(request.into()).await
    }

    async fn send_inner<T>(&self, request: Request) -> Result<T>
    where
        T: Decode,
        T::Error: Into<Error>,
    {
        let tx = self.tx.clone();

        // Create a new channel for receiving the response
        let (resp_tx, resp_rx) = oneshot::channel();

        match tx
            .send(Cmd {
                bytes: Bytes::from(request.buf.into_bytes()),
                resp: resp_tx,
            })
            .await
        {
            Ok(_) => {
                let resp = resp_rx.await.unwrap()?;
                let val = T::decode(&resp).map_err(|e| e.into())?;
                Ok(val)
            }
            Err(_) => Err(Error(ErrorKind::SendError)),
        }
    }

    pub(crate) fn handle_error<E>(&self, error: E)
    where
        E: Into<Error>,
    {
        let inner = self.inner.read().unwrap();
        inner.handler.error(self.clone(), error.into());
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
        let mut req = RequestBuilder::new("apikeyadd").arg("scope", scope);
        if let Some(lifetime) = lifetime {
            req = req.arg("lifetime", lifetime);
        }
        if let Some(cldbid) = cldbid {
            req = req.arg("cldbid", cldbid);
        }

        self.send(req.build()).await
    }

    /// Delete an apikey. Any apikey owned by the current user can always be deleted. Deleting
    /// apikeys from another user requires `b_virtualserver_apikey_manage`.
    pub async fn apikeydel(&self, id: u64) -> Result<()> {
        let req = RequestBuilder::new("apikeydel").arg("id", id);
        self.send(req.build()).await
    }

    /// Lists all apikeys owned by the user, or of all users using `cldbid`=`(0, true).` Usage
    /// of `cldbid`=... requires `b_virtualserver_apikey_manage`.
    pub async fn apikeylist(
        &self,
        cldbid: Option<(u64, bool)>,
        start: Option<u64>,
        duration: Option<u64>,
        count: bool,
    ) -> Result<List<ApiKey, Pipe>> {
        let mut req = RequestBuilder::new("apikeylist");
        if let Some((cldbid, all)) = cldbid {
            if all {
                req = req.arg("cldbid", "*");
            } else {
                req = req.arg("cldbid", cldbid);
            }
        }
        if let Some(start) = start {
            req = req.arg("start", start);
        }
        if let Some(duration) = duration {
            req = req.arg("duration", duration);
        }

        if count {
            req = req.flag("-count");
        }

        self.send(req).await
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
        let mut req = RequestBuilder::new("banadd");

        if let Some(ip) = ip {
            req = req.arg("ip", ip);
        }
        if let Some(name) = name {
            req = req.arg("name", name);
        }
        if let Some(uid) = uid {
            req = req.arg("uid", uid);
        }
        if let Some(mytsid) = mytsid {
            req = req.arg("mytsid", mytsid);
        }
        if let Some(time) = time {
            req = req.arg("time", time);
        }
        if let Some(banreason) = banreason {
            req = req.arg("banreason", banreason);
        }
        if let Some(lastnickname) = lastnickname {
            req = req.arg("lastnickname", lastnickname);
        }

        self.send(req).await
    }

    /// Sends a text message to all clients on all virtual servers in the TeamSpeak 3
    /// Server instance.
    pub async fn gm(&self, msg: &str) -> Result<()> {
        let req = RequestBuilder::new("gm").arg("msg", msg);
        self.send(req).await
    }

    /// Authenticate with the given data.
    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        let req = RequestBuilder::new("login")
            .arg("client_login_name", username)
            .arg("client_login_password", password);
        self.send(req).await
    }

    /// Deselects the active virtual server and logs out from the server instance.
    pub async fn logout(&self) -> Result<()> {
        let req = RequestBuilder::new("logout");
        self.send(req).await
    }

    /// Send a quit command, disconnecting the client and closing the TCP connection
    pub async fn quit(&self) -> Result<()> {
        let req = RequestBuilder::new("quit");
        self.send(req).await
    }

    pub async fn sendtextmessage(&self, target: TextMessageTarget, msg: &str) -> Result<()> {
        let req = RequestBuilder::new("sendtextmessage")
            .arg("targetmode", target)
            .arg("msg", msg);
        self.send(req).await
    }

    /// Adds one or more clients to the server group specified with sgid. Please note that a
    /// client cannot be added to default groups or template groups.
    pub async fn servergroupaddclient(
        &self,
        sgid: ServerGroupId,
        cldbid: ClientDatabaseId,
    ) -> Result<()> {
        let req = RequestBuilder::new("servergroupaddclient")
            .arg("sgid", sgid)
            .arg("cldbid", cldbid);
        self.send(req).await
    }

    /// Removes one or more clients specified with cldbid from the server group specified with
    /// sgid.  
    pub async fn servergroupdelclient(
        &self,
        sgid: ServerGroupId,
        cldbid: ClientDatabaseId,
    ) -> Result<()> {
        let req = RequestBuilder::new("servergroupdelclient")
            .arg("sgid", sgid)
            .arg("cldbid", cldbid);
        self.send(req).await
    }

    /// Registers for a specified category of events on a virtual server to receive
    /// notification messages. Depending on the notifications you've registered for,
    /// the server will send you a message on every event in the view of your
    /// ServerQuery client (e.g. clients joining your channel, incoming text
    /// messages, server configuration changes, etc). The event source is declared by
    /// the event parameter while id can be used to limit the notifications to a
    /// specific channel.  
    pub async fn servernotifyregister(&self, event: ServerNotifyRegister) -> Result<()> {
        let req = RequestBuilder::new("servernotifyregister").arg("event", event);
        self.send(req).await
    }

    /// Starts the virtual server specified with sid. Depending on your permissions,
    /// you're able to start either your own virtual server only or all virtual
    /// servers in the server instance.  
    pub async fn serverstart<T>(&self, sid: T) -> Result<()>
    where
        T: Into<ServerId>,
    {
        let req = RequestBuilder::new("serverstart").arg("sid", sid.into());
        self.send(req).await
    }

    /// Stops the virtual server specified with sid. Depending on your permissions,
    /// you're able to stop either your own virtual server only or all virtual
    /// servers in the server instance. The reasonmsg parameter specifies a
    /// text message that is sent to the clients before the client disconnects.
    pub async fn serverstop<T>(&self, sid: T) -> Result<()>
    where
        T: Into<ServerId>,
    {
        let req = RequestBuilder::new("serverstop").arg("sid", sid.into());
        self.send(req).await
    }

    /// Switch to the virtualserver (voice) with the given server id
    pub async fn use_sid<T>(&self, sid: T) -> Result<()>
    where
        T: Into<ServerId>,
    {
        let req = RequestBuilder::new("use").arg("sid", sid.into());
        self.send(req).await
    }

    /// Like `use_sid` but instead use_port uses the voice port to connect to the virtualserver
    pub async fn use_port(&self, port: u16) -> Result<()> {
        let req = RequestBuilder::new("use").arg("port", port);
        self.send(req).await
    }

    /// Returns information about the server version
    pub async fn version(&self) -> Result<Version> {
        let req = RequestBuilder::new("version");
        self.send(req).await
    }

    /// Returns information about the query client connected
    pub async fn whoami(&self) -> Result<Response> {
        let req = RequestBuilder::new("whoami");
        self.send(req).await
    }
}
