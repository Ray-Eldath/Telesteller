use std::string;

use bytes::Bytes;
use derive_more::From;
use thiserror::Error;

#[macro_use]
use crate::{get, pub_struct};
use crate::message::Qos;
use crate::util::ext::BoolExt;

macro_rules! get_bit {
    ($pos:expr, $subject:expr) => { ($subject & (0b1000_0000 >> $pos) != 0) };
}

macro_rules! assert_byte {
    ($a:literal to $b:literal of $subject:expr, $expected:literal) => {
        assert!((0..=7).contains(&$a));
        assert!((0..=7).contains(&$b));

        for i in ($a..=$b) {
            // println!("get {} of {:b}  result: {} expected: {}", i, $subject, get_bit!(i, $subject), get_bit!(i, $expected));
            if get_bit!(i, $subject) != get_bit!(i, $expected) {
                return Err(Error::MalformedRequest);
            }
        }
    };
}

macro_rules! unpack {
    ($subject:expr) => {
        match $subject {
            Some(Err(err)) => return Err(err),
            Some(Ok(v)) => Some(v),
            None => None
        }
    };
}

macro_rules! consume_item {
    ($cursor:ident of $bytes:ident) => { {
        $cursor += 2;
        let size = u16(get!(($cursor - 2)..$cursor, $bytes)) as usize;
        $cursor += size;
        get!(($cursor - size)..$cursor, $bytes)
    } };
}

macro_rules! into_text {
    ($type:ident whichis $subject:expr) => {
        match String::from_utf8($subject.into()) {
            Ok(v) => v,
            Err(err) => return Err(Error::NonUTF8Text(TextType::$type, err))
        }
    };
}

#[inline]
fn u16(bytes: &[u8]) -> u16 {
    let len = bytes.len() - 1;
    let mut out: u16 = 0;
    for (i, b) in bytes.iter().enumerate() {
        out += (*b as u16) << (8 * (len - i))
    }
    // println!("received {:?}; result: {}", bytes, out);
    out
}

#[derive(Debug, From)]
pub(crate) enum Request {
    CONNECT(CONNECT),
    SUBSCRIBE(SUBSCRIBE),
    PUBLISH(PUBLISH),
}

impl Request {
    pub(crate) fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        match get!(0, bytes) >> 4 {
            0b0001 => Ok(CONNECT::from_bytes(bytes)?.into()),
            0b1000 => Ok(SUBSCRIBE::from_bytes(bytes)?.into()),
            0b0011 => Ok(PUBLISH::from_bytes(bytes)?.into()),
            _ => return Err(Error::MalformedRequest)
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("malformed request received. corresponding TCP connection will be closed according to MQTT 3.1.1 spec.")]
    MalformedRequest,
    #[error("non-UTF8 text in {0:?} received")]
    NonUTF8Text(TextType, string::FromUtf8Error),
}

#[derive(Debug, PartialEq)]
pub enum TextType {
    ClientId,
    WillTopic,
    Username,
    Topic,
}

impl Qos {
    #[inline]
    fn from_bits(b1: bool, b2: bool) -> Result<Qos, Error> {
        match (b1, b2) {
            (false, false) => Ok(Qos::FireAndForget),
            (false, true) => Ok(Qos::AcknowledgedDeliver),
            (true, false) => Ok(Qos::AssuredDelivery),
            _ => Err(Error::MalformedRequest)
        }
    }

    #[inline]
    fn from_byte(b: &u8) -> Result<Qos, Error> {
        match b {
            0b00 => Ok(Qos::FireAndForget),
            0b01 => Ok(Qos::AcknowledgedDeliver),
            0b10 => Ok(Qos::AssuredDelivery),
            _ => Err(Error::MalformedRequest)
        }
    }
}

pub trait RequestFrame {
    fn from_bytes(bytes: Bytes) -> Result<Self, Error> where Self: Sized;
}

pub_struct!(CONNECT {
    protocol_version: u8,
    clean_session: bool,
    keep_alive: u16,
    client_id: String,
    will: Option<Will>,
    username: Option<String>,
    password: Option<Bytes>,
});

pub_struct!(Will {
    qos: Qos,
    retain: bool,
    topic: String,
    payload: Bytes,
});

impl RequestFrame for CONNECT {
    fn from_bytes(bytes: Bytes) -> Result<Self, Error> {
        assert_byte!(4 to 7 of get!(0, bytes), 0);
        if get!(2..=7, bytes) != [0, 4, 77, 81, 84, 84] { // 00 04 M Q T T
            return Err(Error::MalformedRequest);
        }

        let connect_flags = get!(9, bytes);

        let mut cursor = 12;
        let client_id = into_text!(ClientId whichis consume_item!(cursor of bytes));

        let maybe_will =
            get_bit!(5, connect_flags).if_so_then(|| {
                Ok(Will {
                    qos: Qos::from_bits(get_bit!(3, connect_flags), get_bit!(4, connect_flags))?,
                    retain: get_bit!(2, connect_flags),
                    topic: into_text!(WillTopic whichis consume_item!(cursor of bytes)),
                    payload: Bytes::copy_from_slice(consume_item!(cursor of bytes)),
                })
            });

        let maybe_username =
            get_bit!(0, connect_flags).if_so_then(|| {
                Ok(into_text!(Username whichis consume_item!(cursor of bytes)))
            });
        let maybe_password =
            get_bit!(1, connect_flags).if_so_then(|| {
                Ok(Bytes::copy_from_slice(consume_item!(cursor of bytes)))
            });

        Ok(CONNECT {
            protocol_version: *get!(8, bytes),
            clean_session: get_bit!(6, connect_flags),
            keep_alive: u16(&bytes[10..=11]),
            client_id,
            will: unpack!(maybe_will),
            username: unpack!(maybe_username),
            password: unpack!(maybe_password),
        })
    }
}

pub_struct!(SUBSCRIBE {
    id: u16,
    subscriptions: Vec<(String, Qos)>,
});

impl RequestFrame for SUBSCRIBE {
    fn from_bytes(bytes: Bytes) -> Result<Self, Error> where Self: Sized {
        assert_byte!(4 to 7 of get!(0, bytes), 0b0010);

        let len = bytes.len();
        let mut subscriptions = Vec::new();
        let mut cursor = 4;
        loop {
            if cursor >= len { break; }

            let v = consume_item!(cursor of bytes);
            let topic = into_text!(Topic whichis v);
            subscriptions.push((topic, Qos::from_byte(get!(cursor, bytes))?));
            cursor += 1;
        }

        Ok(SUBSCRIBE {
            id: u16(&bytes[2..=3]),
            subscriptions,
        })
    }
}

pub_struct!(PUBLISH {
    dup: bool,
    qos: Qos,
    retain: bool,
    topic: String,
    id: Option<u16>,
    payload: Bytes,
    raw: Bytes, // in order to route the frame to Subscriptions
});

impl RequestFrame for PUBLISH {
    fn from_bytes(bytes: Bytes) -> Result<Self, Error> where Self: Sized {
        let flags = bytes[0];
        let qos = Qos::from_bits(get_bit!(5, flags), get_bit!(6, flags))?;

        let mut cursor = 2;
        let topic = into_text!(Topic whichis consume_item!(cursor of bytes));
        let maybe_id =
            (qos > Qos::FireAndForget).if_so_then(|| {
                cursor += 2;
                Ok(u16(&bytes[(cursor - 2)..cursor]))
            });

        Ok(PUBLISH {
            dup: get_bit!(4, flags),
            qos,
            retain: get_bit!(7, flags),
            topic,
            id: unpack!(maybe_id),
            payload: Bytes::copy_from_slice(&bytes[cursor..]),
            raw: bytes,
        })
    }
}