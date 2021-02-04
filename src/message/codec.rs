use std::io;

use bytes::{Buf, BytesMut};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder, Framed};

#[macro_use]
use crate::get;
use crate::message::{Request, ResponseFrame};

pub(crate) struct MQTT311;

pub(crate) type Transport = Framed<tokio::net::TcpStream, MQTT311>;

#[derive(Error, Debug)]
pub(crate) enum EncodeError {
    #[error("err {0} occurred while marshalling a Response.")]
    Marshalling(#[from] super::response::Error),
    #[error("IOError {0} occurred while reading the bitstream.")]
    IO(#[from] std::io::Error),
}

impl Encoder<Box<dyn ResponseFrame + Sync + Send>> for MQTT311 {
    type Error = EncodeError;

    fn encode(&mut self, item: Box<dyn ResponseFrame + Sync + Send>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.to_bytes(dst).map_err(|e| e.into())
    }
}

#[derive(Error, Debug)]
pub(crate) enum DecodeError {
    #[error("err {0} occurred while parsing frame.")]
    Parsing(#[from] super::request::Error),
    #[error("IOError {0} occurred while reading the bitstream.")]
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
            cursor += 1;

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