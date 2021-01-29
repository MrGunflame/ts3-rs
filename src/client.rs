use bytes::Bytes;
use std::collections::HashMap;
use std::convert::From;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::result;
use std::str::from_utf8;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::sync::{mpsc, oneshot};
use tokio::task::spawn;
use tokio::time::sleep;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    TS3 { id: usize, msg: String },
    SendError,
}

impl Error {
    fn ok(&self) -> bool {
        use Error::*;
        match self {
            TS3 { id, msg: _ } => *id == 0,
            _ => false,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Error::*;
        write!(
            f,
            "{}",
            match self {
                IO(err) => format!("{}", err),
                TS3 { id, msg } => format!("TS3 Error {}: {}", id, msg),
                SendError => "SendError".to_owned(),
            }
        )
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(err)
    }
}

// Read a error from a raw server response
impl From<RawResp> for Error {
    fn from(mut resp: RawResp) -> Error {
        Error::TS3 {
            id: resp.items[0]
                .remove("id")
                .unwrap()
                .unwrap()
                .parse()
                .unwrap(),
            msg: resp.items[0].remove("msg").unwrap().unwrap(),
        }
    }
}

struct Cmd {
    bytes: Bytes,
    resp: oneshot::Sender<Result<RawResp>>,
}

/// A Client used to send commands to the serverquery interface.
#[derive(Clone)]
pub struct Client {
    tx: mpsc::Sender<Cmd>,
}

impl Client {
    /// Create a new connection
    pub async fn new<A: ToSocketAddrs>(addr: A) -> Result<Client> {
        let (tx, mut rx) = mpsc::channel::<Cmd>(32);

        let stream = TcpStream::connect(addr).await?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // read_tx and read_rx are used to communicate between the read and the write
        // thread
        let (read_tx, mut read_rx) = mpsc::channel(32);

        // Read task
        spawn(async move {
            loop {
                let mut buf = Vec::new();
                if let Err(err) = reader.read_until(b'\r', &mut buf).await {
                    println!("{}", err);
                    continue;
                }

                // Remove the last two bytes '\n' and '\r'
                buf.truncate(buf.len() - 2);

                let resp = RawResp::from(buf.as_slice());
                match resp.is_error() {
                    true => {
                        let _ = read_tx.send((RawResp::new(), Error::from(resp))).await;
                    }
                    false => {
                        // Read another line
                        buf.clear();
                        if let Err(err) = reader.read_until(b'\r', &mut buf).await {
                            eprintln!("{}", err);
                            continue;
                        }

                        let err = RawResp::from(buf.as_slice());
                        let _ = read_tx.send((resp, Error::from(err))).await;
                    }
                }
            }
        });

        spawn(async move {
            while let Some(cmd) = rx.recv().await {
                // Write the command string
                if let Err(err) = writer.write(&cmd.bytes).await {
                    let _ = cmd.resp.send(Err(err.into()));
                    continue;
                }

                // Write a '\n' to send the command
                if let Err(err) = writer.write(&[b'\n']).await {
                    let _ = cmd.resp.send(Err(err.into()));
                    continue;
                }

                // Wait for the response from the reader thread
                let (resp, err) = read_rx.recv().await.unwrap();

                let _ = cmd.resp.send(match err.ok() {
                    true => Ok(resp),
                    false => Err(err),
                });
            }
        });

        // Keepalive loop
        let tx2 = tx.clone();
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

        Ok(Client { tx })
    }

    /// Send a raw command directly to the server
    pub async fn send(&self, cmd: String) -> Result<RawResp> {
        let tx = self.tx.clone();

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
                resp.unwrap()
            }
            Err(_) => Err(Error::SendError),
        }
    }
}

// TS3 Commands go here
impl Client {
    /// Authenticate with the given data.
    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        self.send(format!(
            "login client_login_name={} client_login_password={}",
            username, password
        ))
        .await?;
        Ok(())
    }

    /// Send a quit command, disconnecting the client and closing the TCP connection
    pub async fn quit(&self) -> Result<()> {
        self.send("quit".to_owned()).await?;
        Ok(())
    }

    /// Switch to the virtualserver (voice) with the given server id
    pub async fn use_sid(&self, sid: usize) -> Result<()> {
        self.send(format!("use sid={}", sid)).await?;
        Ok(())
    }
}

/// RawResp contains all data returned from the server
/// When the items vector contains multiple entries, the server returned a list.
/// Otherwise only a single item will be in the vector
/// The HashMap contains all key-value pairs, but values are optional
pub struct RawResp {
    pub items: Vec<HashMap<String, Option<String>>>,
}

impl RawResp {
    fn new() -> RawResp {
        RawResp { items: Vec::new() }
    }

    // Returns whether the decoded response is the server error response
    // This is true when a key named "error" exists in the map
    fn is_error(&self) -> bool {
        match self.items.get(0) {
            Some(map) => map.contains_key("error"),
            None => false,
        }
    }
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
