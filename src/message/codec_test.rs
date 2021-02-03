#![allow(non_snake_case)]

use std::io::{Cursor, Read, Seek};
use std::rc::Rc;

use bytes::Buf;
use futures::SinkExt;
use hex_literal::hex;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};

use super::codec::*;
use super::Qos;
use super::response::*;

#[tokio::test]
async fn test_read_CONNECT_SUBSCRIBE_PUBLISH() {
    let stream = Cursor::new(hex!("
                10 72 00 04 4d 51 54 54 04 36 01 2c 00 31 33 65
                32 36 63 34 36 35 2d 31 33 31 39 2d 34 65 34 32
                2d 38 35 33 35 2d 31 37 63 63 63 30 31 66 65 63
                39 39 31 36 31 31 37 36 32 30 31 39 35 35 38 00
                0e 2f 74 65 73 74 77 69 6c 6c 2f 77 69 6c 6c 00
                23 64 65 76 69 63 65 20 6e 6f 77 20 67 6f 20 75
                6e 67 72 61 63 65 66 75 6c 6c 79 20 6f 66 66 6c
                69 6e 65 2e
                82 13 a1 12 00 0e 2f 74 65 73 74 77 69 6c 6c 2f
                77 69 6c 6c 02
                30 0a 00 05 2f 61 62 63 64 31 32 33
            "));
    let mut transport = FramedRead::new(stream, MQTT311);
    while let Some(request) = transport.next().await {
        match request {
            Ok(request) => println!("{:?}", request),
            Err(e) => panic!("Err: {:?}", e),
        }
    }
}

#[tokio::test]
async fn test_write_CONNACK_SUBACK() {
    let stream: Cursor<&mut [u8]> = Cursor::default();
    let mut transport = FramedWrite::new(stream, MQTT311);
    transport.send(Box::new(CONNACK {
        session_present: true,
        return_code: CONNACKReturnCode::Accepted,
    })).await;
    transport.send(Box::new(SUBACK {
        id: 41235,
        granted_qos: vec![Some(Qos::AcknowledgedDeliver), None, Some(Qos::FireAndForget), Some(Qos::AssuredDelivery)],
    })).await;

    assert_eq!(transport.write_buffer().to_vec(), &hex!("
        20 02 01 00
        90 06 a1 13 01 80 00 02
    ")[..]);
}