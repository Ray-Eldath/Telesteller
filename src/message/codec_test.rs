#![allow(non_snake_case)]

use tokio_util::codec::FramedRead;
use tokio_stream::StreamExt;
use std::io::Cursor;
use hex_literal::hex;
use super::codec::*;

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