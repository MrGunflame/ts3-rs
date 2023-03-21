use std::fmt::{self, Display, Formatter};

use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ServerId(pub u64);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ClientId(pub u64);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ClientDatabaseId(pub u64);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ChannelId(pub u64);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ServerGroupId(pub u64);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ChannelGroupId(pub u64);

macro_rules! id_impls {
    ($($t:ty),*$(,)?) => {
        $(
            impl Display for $t {
                #[inline]
                fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                    Display::fmt(&self.0, f)
                }
            }

            impl Encode for $t {
                #[inline]
                fn encode(&self, buf: &mut String) {
                    self.0.encode(buf)
                }
            }

            impl Decode for $t {
                type Error = <u64 as Decode>::Error;

                #[inline]
                fn decode(buf: &[u8]) -> Result<Self, Self::Error> {
                    u64::decode(buf).map(Self)
                }
            }
        )*
    };
}

id_impls! {
    ServerId,
    ClientId,
    ClientDatabaseId,
    ChannelId,
    ServerGroupId,
    ChannelGroupId,
}
