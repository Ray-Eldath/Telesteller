#![allow(non_snake_case)]

use bytes::BytesMut;
use hex_literal::hex;

use super::Qos;
use super::response::*;

fn test_success(frame: impl ResponseFrame, expected: &[u8]) {
    let mut bytes = BytesMut::new();
    frame.to_bytes(&mut bytes);
    let bytes = bytes.freeze();
    println!("frame - length: {}  bytes: {:x}", bytes.len(), bytes);

    assert_eq!(format!("{:x}", bytes),
               expected.iter()
                   .map(|e| format!("{:02x}", e)).collect::<Vec<_>>()
                   .join(""));
}

macro_rules! test_success {
    ($frame:expr, $expected:literal) => {
        test_success($frame, hex!($expected)[..].into());
    };
}

#[test]
fn test_CONNACK() {
    test_success!(CONNACK {
        session_present: true,
        return_code: CONNACKReturnCode::Accepted,
    }, "20 02 01 00");
}

#[test]
fn test_SUBACK() {
    test_success!(SUBACK {
        id: 41235,
        granted_qos: vec![Some(Qos::FireAndForget)],
    }, "90 03 a1 13 00");
}

#[test]
fn test_UNSUBACK() {
    test_success!(UNSUBACK {
        id: 18633
    }, "b0 02 48 c9");
}

#[test]
fn test_PINGRESP() {
    test_success!(PINGRESP {}, "d0 00");
}