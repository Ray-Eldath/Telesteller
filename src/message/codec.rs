use tracing::debug;
use tokio_util::codec::{Decoder, Encoder, Framed};
use bytes::{BytesMut, Bytes, Buf};
use derive_more::From;
use std::io;
use thiserror::Error;
use crate::message::{Request, request::Error};
#[macro_use]
use crate::get;

pub(crate) struct MQTT311;

#[derive(Error, Debug)]
pub(crate) enum DecodeError {
    #[error("err {0} occurred while parsing frame.")]
    Parsing(#[from] super::request::Error),
    #[error("IOError {0} occurred while reading the frame.")]
    IO(#[from] std::io::Error),
}

impl Decoder for MQTT311 {
    type Item = Request;
    type Error = DecodeError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 2 {
            return Ok(None);
        }

        // Decode variable-length Remaining Length field, according to MQTT311 spec 2.2.3
        let mut cursor = 1;
        let mut multiplier = 1;
        let mut length = 0;
        loop {
            let byte = usize::from(*get!(cursor, src));
            length += (byte & 127) * multiplier;
            if multiplier > 128 * 128 * 128 {
                return Err(io::Error::new(io::ErrorKind::InvalidData,
                                          format!("Frame is too large.")).into());
            }
            multiplier *= 128;

            if (byte & 128) == 0 { break; }
        }
        length += 2;
        // println!("length: {}  len: {}", length, src.len());
        // println!("{:?}", src.to_vec());

        if src.len() < length {
            src.reserve(length - src.len());
            return Ok(None);
        } else {
            let data = get!(..length, src).to_vec();
            src.advance(length);

            Request::from_bytes(data.into())
                .map(|r| Some(r))
                .map_err(|e| DecodeError::Parsing(e))
        }
    }
}