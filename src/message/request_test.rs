#![allow(non_snake_case)]

use bytes::Bytes;
use hex_literal::hex;
use super::request::*;

macro_rules! hex_bytes {
    ($data:literal) => {
        hex!($data)[..].into()
    };
}

#[test]
fn test_CONNECT() {
    let parsed = CONNECT::from_bytes(hex_bytes!("
        10 72 00 04 4d 51 54 54 04 36 01 2c 00 31 33 65
        32 36 63 34 36 35 2d 31 33 31 39 2d 34 65 34 32
        2d 38 35 33 35 2d 31 37 63 63 63 30 31 66 65 63
        39 39 31 36 31 31 37 36 32 30 31 39 35 35 38 00
        0e 2f 74 65 73 74 77 69 6c 6c 2f 77 69 6c 6c 00
        23 64 65 76 69 63 65 20 6e 6f 77 20 67 6f 20 75
        6e 67 72 61 63 65 66 75 6c 6c 79 20 6f 66 66 6c
        69 6e 65 2e
    ")).unwrap();
    println!("{:?}", parsed);

    assert_eq!(parsed.clean_session, true);
    assert_eq!(parsed.keep_alive, 300);
    assert_eq!(parsed.client_id, "3e26c465-1319-4e42-8535-17ccc01fec991611762019558");
    assert_eq!(parsed.will, Some(Will {
        qos: Qos::AssuredDelivery,
        retain: true,
        topic: "/testwill/will".to_string(),
        payload: "device now go ungracefully offline.".into(),
    }));
    assert_eq!(parsed.username, None);
    assert_eq!(parsed.password, None);
}

#[test]
fn test_CONNECT_malformed() {
    let malformed1 = CONNECT::from_bytes(hex_bytes!("
        10 76 00 04 4d 51 54 54 04 36 01 2c 00 31 33 65
        32 36 63 34 36 35 2d 31 33 31 39 2d 34 65 34 32
    ")).unwrap_err();
    let malformed2 = CONNECT::from_bytes(hex_bytes!("
        10 72 00 04 4d 51 54 54 04 36 01 2c 00 31 33 65
        32 36 63 34 36 35 2d 31 33 31 39 2d 34 65 34 32
        2d 38 35 33 35 2d 31 37 63 63 63 30 31 66 65 63
        39 39 31 36 31 31 37 36 32 30 31 39 35 35 38
    ")).unwrap_err(); // here we set Will flag to 1 but left out Will.
    println!("{:?}", malformed1);
    println!("{:?}", malformed2);

    assert_eq!(malformed1, Error::MalformedRequest);
    assert_eq!(malformed2, Error::MalformedRequest);
}

#[test]
fn test_CONNECT_non_utf8() {
    // ClientId
    if let Error::NonUTF8Text(type1, _) = CONNECT::from_bytes(hex_bytes!("
        10 72 00 04 4d 51 54 54 04 12 01 2c 00 04 00 9F
        92 96
    ")).unwrap_err() { // construct a CONNECT without Will, but contains a non UTF-8 character in ClientId.
        println!("type1: {:?}", type1);
        assert_eq!(type1, TextType::ClientId);
    } else {
        assert!(false);
    }

    // WillTopic
    if let Error::NonUTF8Text(type2, _) = CONNECT::from_bytes(hex_bytes!("
        10 72 00 04 4d 51 54 54 04 36 01 2c 00 31 33 65
        32 36 63 34 36 35 2d 31 33 31 39 2d 34 65 34 32
        2d 38 35 33 35 2d 31 37 63 63 63 30 31 66 65 63
        39 39 31 36 31 31 37 36 32 30 31 39 35 35 38 00
        04 00 9F 92 96 00 01 FF
    ")).unwrap_err() { // construct a CONNECT with Will, and contains a non UTF-8 character in WillTopic.
        println!("type2: {:?}", type2);
        assert_eq!(type2, TextType::WillTopic);
    } else {
        assert!(false);
    }
}

#[test]
fn test_SUBSCRIBE() {
    let parsed = SUBSCRIBE::from_bytes(hex_bytes!("
        82 13 a1 12 00 0e 2f 74 65 73 74 77 69 6c 6c 2f
        77 69 6c 6c 02
    ")).unwrap();
    println!("{:?}", parsed);

    assert_eq!(parsed.id, 41234);
    assert_eq!(parsed.topic, "/testwill/will");
    assert_eq!(parsed.qos, Qos::AssuredDelivery);
}

#[test]
fn test_SUBSCRIBE_malformed() {
    let malformed1 = SUBSCRIBE::from_bytes(hex_bytes!("
        83 13 a1 12 00 0e 2f 74 65 73 74 77 69 6c 6c 2f
        77 69 6c 6c 02
    ")).unwrap_err();
    println!("{:?}", malformed1);

    assert_eq!(malformed1, Error::MalformedRequest);
}

#[test]
fn test_PUBLISH() {
    // PUBLISH with Id (Qos = 2)
    let parsed1 = PUBLISH::from_bytes(hex_bytes!("
        34 0c 00 05 2f 61 62 63 64 a1 16 31 32 33
    ")).unwrap();
    println!("{:?}", parsed1);

    assert_eq!(parsed1.dup, false);
    assert_eq!(parsed1.qos, Qos::AssuredDelivery);
    assert_eq!(parsed1.retain, false);
    assert_eq!(parsed1.topic, "/abcd");
    assert_eq!(parsed1.id, Some(41238));
    assert_eq!(parsed1.payload, Bytes::from("123"));

    // PUBLISH without Id (Qos = 0)
    let parsed2 = PUBLISH::from_bytes(hex_bytes!("
        30 0c 00 05 2f 61 62 63 64 31 32 33
    ")).unwrap();
    println!("{:?}", parsed2);

    assert_eq!(parsed2.id, None);
    assert_eq!(parsed2.payload, Bytes::from("123"));

    // PUBLISH without Id (Qos == 0) and Payload
    let parsed3 = PUBLISH::from_bytes(hex_bytes!("
        30 0c 00 05 2f 61 62 63 64
    ")).unwrap();
    println!("{:?}", parsed3);

    assert_eq!(parsed2.id, None);
    assert!(parsed3.payload.is_empty());
}