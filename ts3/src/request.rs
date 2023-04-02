use crate::{
    types::{ChannelId, ClientId},
    Encode,
};

/// An encoded request buffer.
#[derive(Clone, Debug)]
pub struct Request {
    pub(crate) buf: String,
}

/// A builder type for a [`Request`].
#[derive(Clone, Debug, Default)]
pub struct RequestBuilder {
    buf: String,
}

impl RequestBuilder {
    /// Creates a new `RequestBuilder`.
    #[inline]
    pub fn new<T>(command: T) -> Self
    where
        T: ToString,
    {
        Self {
            buf: command.to_string(),
        }
    }

    /// Appends an key-value argument to the request.
    pub fn arg<K, V>(mut self, key: K, value: V) -> Self
    where
        K: AsRef<str>,
        V: Encode,
    {
        self.buf += " ";
        self.buf += key.as_ref();
        self.buf += "=";
        value.encode(&mut self.buf);
        self
    }

    pub fn flag<T>(mut self, flag: T) -> Self
    where
        T: AsRef<str>,
    {
        self.buf += " ";
        self.buf += flag.as_ref();
        self
    }

    /// Consumes this `RequestBuilder`, returning the constructed [`Request`].
    #[inline]
    pub fn build(self) -> Request {
        Request { buf: self.buf }
    }
}

impl From<RequestBuilder> for Request {
    #[inline]
    fn from(value: RequestBuilder) -> Self {
        value.build()
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

impl Encode for ServerNotifyRegister {
    fn encode(&self, buf: &mut String) {
        match self {
            Self::Server => *buf += "server",
            Self::Channel(cid) => *buf += &format!("channel id={}", cid),
            Self::TextServer => *buf += "textserver",
            Self::TextChannel => *buf += "textchannel",
            Self::TextPrivate => *buf += "textprivate",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextMessageTarget {
    Client(ClientId),
    Channel,
    Server,
}

impl Encode for TextMessageTarget {
    fn encode(&self, buf: &mut String) {
        match self {
            Self::Client(clid) => *buf += &format!("1 target={}", clid),
            Self::Channel => *buf += "2",
            Self::Server => *buf += "3",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RequestBuilder;

    #[test]
    fn test_request_builder() {
        let cmd = RequestBuilder::new("testcmd");
        assert_eq!(cmd.clone().buf, "testcmd");

        let cmd = cmd.arg("hello", "world");
        assert_eq!(cmd.clone().buf, "testcmd hello=world");

        let cmd = cmd.arg("test", "1234");
        assert_eq!(cmd.clone().buf, "testcmd hello=world test=1234");
    }
}
