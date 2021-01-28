use std::{fmt, string};
use bytes::Bytes;
use thiserror::Error;

macro_rules! get_bit {
    ($pos:expr, $subject:expr) => { ($subject & (0b1000_0000 >> $pos) != 0) };
}

macro_rules! get {
    ($pos:expr, $subject:ident) => {
        $subject.get($pos).ok_or(Error::MalformedRequest)?
    };
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

trait BoolExt {
    fn if_so<F, R>(self, f: F) -> Option<R> where F: FnMut() -> R;
}

impl BoolExt for bool {
    fn if_so<F, R>(self, mut f: F) -> Option<R> where F: FnMut() -> R {
        if self {
            Some(f())
        } else {
            None
        }
    }
}

pub(crate) struct Message {}

struct Frame {}

#[derive(Error, Debug, PartialEq)]
pub(crate) enum Error {
    #[error("malformed request received. corresponding TCP connection will be closed according to MQTT 3.1.1 spec.")]
    MalformedRequest,
    #[error("non-UTF8 text in {0:?} received")]
    NonUTF8Text(TextType, string::FromUtf8Error),
}

#[derive(Debug, PartialEq)]
pub(crate) enum TextType {
    ClientId,
    WillTopic,
    Username,
    Topic,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Qos {
    FireAndForget,
    AcknowledgedDeliver,
    AssuredDelivery,
}

impl Qos {
    fn from_bits(b1: bool, b2: bool) -> Qos {
        match (b1, b2) {
            (false, false) => Qos::FireAndForget,
            (false, true) => Qos::AcknowledgedDeliver,
            (true, false) => Qos::AssuredDelivery,
            _ => panic!("unexpected bit {}{} incoming Qos::from_bits", bit!(b1), bit!(b2))
        }
    }

    fn from_byte(b: &u8) -> Qos {
        match b {
            0b00 => Qos::FireAndForget,
            0b01 => Qos::AcknowledgedDeliver,
            0b10 => Qos::AssuredDelivery,
            _ => panic!("unexpected byte {:b} incoming Qos::from_byte", b)
        }
    }
}

pub(crate) trait RequestFrame {
    fn from_bytes(bytes: Bytes) -> Result<Self, Error> where Self: Sized;
}

pub_struct!(CONNECT {
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
        let connect_flags = get!(9, bytes);

        let mut cursor = 12;
        let client_id = into_text!(ClientId whichis consume_item!(cursor of bytes));

        let maybe_will =
            get_bit!(5, connect_flags).if_so(|| {
                Ok(Will {
                    qos: Qos::from_bits(get_bit!(3, connect_flags), get_bit!(4, connect_flags)),
                    retain: get_bit!(2, connect_flags),
                    topic: into_text!(WillTopic whichis consume_item!(cursor of bytes)),
                    payload: Bytes::copy_from_slice(consume_item!(cursor of bytes)),
                })
            });

        let maybe_username =
            get_bit!(0, connect_flags).if_so(|| {
                Ok(into_text!(Username whichis consume_item!(cursor of bytes)))
            });
        let maybe_password =
            get_bit!(1, connect_flags).if_so(|| {
                Ok(Bytes::copy_from_slice(consume_item!(cursor of bytes)))
            });

        Ok(CONNECT {
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
            qos: Qos::from_byte(&bytes[cursor]),
        })
    }
}