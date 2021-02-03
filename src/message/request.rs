use std::{fmt, string};
use bytes::Bytes;
use thiserror::Error;
use derive_more::{From, Into};
use crate::util::ext::BoolExt;
#[macro_use]
use crate::get;

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
        let size = usize(get!(($cursor - 2)..$cursor, $bytes));
        $cursor += size;
        get!(($cursor - size)..$cursor, $bytes)
    } };
}

macro_rules! bit {
    ($subject:ident) => {
        if $subject { "1" } else { "0" }
    };
}

macro_rules! pub_struct {
    ($name:ident { $($field:ident: $t:ty,)*} ) => {
        #[derive(Debug, PartialEq)]
        pub(crate) struct $name {
            $(pub(crate) $field: $t),*
        }
    }
}

macro_rules! into_text {
    ($type:ident whichis $subject:expr) => {
        match String::from_utf8($subject.into()) {
            Ok(v) => v,
            Err(err) => return Err(Error::NonUTF8Text(TextType::$type, err))
        }
    };
}

fn usize(bytes: &[u8]) -> usize {
    let len = bytes.len() - 1;
    let mut out: usize = 0;
    for (i, b) in bytes.iter().enumerate() {
        out += (*b as usize) << (8 * (len - i))
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

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Qos {
    FireAndForget,
    AcknowledgedDeliver,
    AssuredDelivery,
}

impl Qos {
    fn from_bits(b1: bool, b2: bool) -> Result<Qos, Error> {
        match (b1, b2) {
            (false, false) => Ok(Qos::FireAndForget),
            (false, true) => Ok(Qos::AcknowledgedDeliver),
            (true, false) => Ok(Qos::AssuredDelivery),
            _ => Err(Error::MalformedRequest)
        }
    }

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
    keep_alive: usize,
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
            keep_alive: usize(&bytes[10..=11]),
            client_id,
            will: unpack!(maybe_will),
            username: unpack!(maybe_username),
            password: unpack!(maybe_password),
        })
    }
}

pub_struct!(SUBSCRIBE {
    id: usize,
    topic: String,
    qos: Qos,
});

impl RequestFrame for SUBSCRIBE {
    fn from_bytes(bytes: Bytes) -> Result<Self, Error> where Self: Sized {
        assert_byte!(4 to 7 of get!(0, bytes), 0b0010);

        let mut cursor = 4;
        Ok(SUBSCRIBE {
            id: usize(&bytes[2..=3]),
            topic: into_text!(Topic whichis consume_item!(cursor of bytes)),
            qos: Qos::from_byte(&bytes[cursor])?,
        })
    }
}

pub_struct!(PUBLISH {
    dup: bool,
    qos: Qos,
    retain: bool,
    topic: String,
    id: Option<usize>,
    payload: Bytes,
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
                Ok(usize(&bytes[(cursor - 2)..cursor]))
            });

        Ok(PUBLISH {
            dup: get_bit!(4, flags),
            qos,
            retain: get_bit!(7, flags),
            topic,
            id: unpack!(maybe_id),
            payload: Bytes::copy_from_slice(&bytes[cursor..]),
        })
    }
}