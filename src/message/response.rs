use bytes::{BufMut, BytesMut};
use thiserror::Error;

use crate::pub_struct;

use super::Qos;

macro_rules! set_bit {
    ($pos:literal to $value:expr, $subject:expr) => {
        if $value { $subject | (0b1000_0000 >> $pos) }
        else      { $subject & (0b0111_1111) >> $pos }
    };
}

#[inline]
// TODO: anything like pub(test)?
pub(super) fn put_length(x: usize, dst: &mut BytesMut) {
    let mut x = x;
    loop {
        let mut encoded = x % 128;
        x /= 128;
        if x > 0 {
            encoded = encoded | 128;
        }
        dst.put_u8(encoded as u8);

        if x <= 0 { break; }
    }
}

#[inline]
fn write_frame(header: u8, data: &[u8], dst: &mut BytesMut) {
    let len = data.len();
    dst.reserve(len + 8);

    dst.put_u8(header);
    put_length(len, dst);
    dst.extend_from_slice(data);
}

#[derive(Error, Debug, PartialEq)]
pub enum Error {}

pub trait ResponseFrame {
    fn to_bytes(&self, dst: &mut BytesMut) -> Result<(), Error>;
}

pub_struct!(CONNACK {
    session_present: bool,
    return_code: CONNACKReturnCode,
});

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) enum CONNACKReturnCode {
    Accepted = 0,
    UnacceptableProtocol = 1,
    IdentifierRejected = 2,
    ServerUnavailable = 3,
    BadUsernameOrPassword = 4,
    NotAuthorized = 5,
}

impl ResponseFrame for CONNACK {
    fn to_bytes(&self, dst: &mut BytesMut) -> Result<(), Error> {
        write_frame(32, &[set_bit!(7 to self.session_present, 0), self.return_code as u8], dst);

        Ok(())
    }
}

pub_struct!(SUBACK {
    id: u16,
    granted_qos: Vec<Option<Qos>>,
});

pub_struct!(UNSUBACK {
    id: u16,
});

impl ResponseFrame for UNSUBACK {
    fn to_bytes(&self, dst: &mut BytesMut) -> Result<(), Error> {
        write_frame(176, &self.id.to_be_bytes(), dst);

        Ok(())
    }
}

impl ResponseFrame for SUBACK {
    fn to_bytes(&self, dst: &mut BytesMut) -> Result<(), Error> {
        let mut payload = self.id.to_be_bytes().to_vec();
        for qos in self.granted_qos.iter().as_ref() {
            let qos_byte = match qos {
                Some(qos) => *qos as u8,
                None => 128
            };
            payload.put_u8(qos_byte);
        }
        write_frame(144, payload.as_slice(), dst);

        Ok(())
    }
}

pub_struct!(PINGRESP {});

impl ResponseFrame for PINGRESP {
    fn to_bytes(&self, dst: &mut BytesMut) -> Result<(), Error> {
        write_frame(208, &[], dst);

        Ok(())
    }
}