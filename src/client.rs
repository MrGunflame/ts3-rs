use std::io::{Read, Write};
use std::net::SocketAddr;
use std::net::TcpStream;
use std::result;

struct Error {
    id: u32,
    text: String,
}

type Result<T> = result::Result<T, Error>;

pub enum ConnectMode {
    TCP,
    SSH,
}

pub struct Client {
    mode: ConnectMode,
    stream: Option<TcpStream>,
    addr: String,
}

pub struct Whoami {
    virtualserver_status: String,
    virtualserver_unique_identifier: String,
    virtualserver_port: u16,
    virtualserver_id: u32,
    client_id: u32,
    client_channel_id: u32,
    client_nickname: String,
    client_database_id: u32,
    client_login_name: String,
    client_unique_identifier: String,
    client_origin_server_id: u8,
}

impl Whoami {
    fn from(vec: Vec<DecodedField>) -> Whoami {
        let (
            virtualserver_status,
            virtualserver_unique_identifier,
            client_nickname,
            client_login_name,
            client_unique_identifier,
        ): (String, String, String, String, String);
        let (
            virtualserver_port,
            virtualserver_id,
            client_id,
            cient_channel_id,
            client_database_id,
            client_origin_server_id,
        ): (u16, u32, u32, u32, u32, u8);

        for f in DecodedField {
            match f.name {}
        }

        Whoami {}
    }
}

impl Client {
    pub fn new(addr: A) -> Client {
        Client {
            mode: ConnectMode::TCP,
            stream: None,
            addr: addr,
        }
    }

    fn rw(&mut self, buf: &[u8]) -> Result<()> {
        self.stream.as_ref().unwrap().write(buf).unwrap();

        Ok(())
    }

    pub fn login(username: String, password: String) {}

    pub fn whoami() {}

    pub fn servergroupaddclient(sgid: u32, cldbid: u32) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
struct DecodedField {
    pub name: String,
    pub value: String,
}

impl DecodedField {
    fn new() -> DecodedField {
        DecodedField {
            name: "".to_owned(),
            value: "".to_owned(),
        }
    }

    fn push_name(&mut self, s: &str) {
        self.name.push_str(s);
    }

    fn push_value(&mut self, s: &str) {
        self.value.push_str(s);
    }
}

// read_buf reads the buffer into a vec of key-value pairs
fn read_buf(buf: &[u8]) -> Vec<DecodedField> {
    let mut decf: Vec<DecodedField> = Vec::new();
    decf.push(DecodedField::new());

    let mut val = false;
    for b in buf {
        match b {
            0x20 => {
                val = false;
                decf.push(DecodedField::new());
            }
            0x3D => {
                val = true;
            }
            _ => {
                let mut last = decf.last_mut().unwrap();
                match val {
                    false => last.push_name(std::str::from_utf8(&[*b]).unwrap()),
                    true => last.push_value(std::str::from_utf8(&[*b]).unwrap()),
                };
            }
        }
    }

    decf
}

mod tests {
    use super::*;

    #[test]
    fn test_read_buf() {
        assert_eq!(
            read_buf(String::from("virt=0 test=1 ok").as_bytes()),
            vec![
                DecodedField {
                    name: "virt".to_owned(),
                    value: "0".to_owned(),
                },
                DecodedField {
                    name: "test".to_owned(),
                    value: "1".to_owned(),
                },
                DecodedField {
                    name: "ok".to_owned(),
                    value: "".to_owned(),
                },
            ]
        );
    }
}
