// mod client;
use std::convert::From;
use std::io::{self, BufRead, BufReader};
use std::net::{TcpStream, ToSocketAddrs};
use std::result;

struct Error {
    io: Option<std::io::Error>,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self { io: Some(err) }
    }
}

type Result<T> = result::Result<T, Error>;

trait TSEncoded {
    fn from(buf: &[u8]) -> Self;
    fn into() -> [u8];
}

// pub use client::Client;

struct Client {
    rx: BufReader<TcpStream>,
    tx: TcpStream,
}

impl Client {
    fn new<A: ToSocketAddrs>(addr: A) -> Result<Client> {
        let (tx, rx) = Self::new_stream(addr)?;
        Ok(Client { tx, rx })
    }

    fn new_stream<A: ToSocketAddrs>(addr: A) -> Result<(TcpStream, BufReader<TcpStream>)> {
        let sock = TcpStream::connect(addr)?;

        let mut reader = BufReader::new(sock.try_clone()?);
        let mut buf = Vec::new();
        reader.read_until(b'\r', &mut buf)?;

        buf.clear();

        Ok((sock, reader))
    }

    fn read(&mut self) {}

    // fn write<T: AsRef<str>>(&mut self, cmd: T) {
    //     writeln!(&mut self.tx, "{}", cmd);
    // }

    // fn quit(&mut self) -> Result<()> {
    //     writeln!(&mut self.tx, "quit")?;
    // }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
