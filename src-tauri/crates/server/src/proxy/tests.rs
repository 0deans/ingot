use super::protocol::*;
use std::io;

#[tokio::test]
async fn test_varint_roundtrip() {
    let test_values = vec![0, 1, 2, 127, 128, 255, 256, i32::MAX, i32::MIN, -1];
    for val in test_values {
        let mut buf = Vec::new();
        encode_varint(val, &mut buf);

        let mut cursor = io::Cursor::new(buf);
        let decoded = read_varint(&mut cursor)
            .await
            .expect("failed to decode varint");
        assert_eq!(val, decoded);
    }
}

#[tokio::test]
async fn varint_rejects_excess_bytes_before_consuming_next_field() {
    let mut cursor = io::Cursor::new([0x80, 0x80, 0x80, 0x80, 0x80, 0x01]);
    let error = read_varint(&mut cursor).await.unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(cursor.position(), 5);
}

#[tokio::test]
async fn varint_rejects_payload_overflow_in_fifth_byte() {
    for final_byte in [0x10, 0x1f, 0x70, 0x7f] {
        let mut cursor = io::Cursor::new([0xff, 0xff, 0xff, 0xff, final_byte]);
        let error = read_varint(&mut cursor).await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}

#[tokio::test]
async fn varint_preserves_truncated_input_error() {
    let mut cursor = io::Cursor::new([0x80, 0x80]);
    let error = read_varint(&mut cursor).await.unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
}

#[tokio::test]
async fn test_handshake_decode() {
    let mut body = Vec::new();
    encode_varint(0x00, &mut body);
    encode_varint(765, &mut body);
    let addr = "localhost";
    encode_varint(addr.len() as i32, &mut body);
    body.extend_from_slice(addr.as_bytes());
    body.extend_from_slice(&25565u16.to_be_bytes());
    encode_varint(2, &mut body);

    let mut wire_packet = Vec::new();
    encode_varint(body.len() as i32, &mut wire_packet);
    wire_packet.extend_from_slice(&body);

    let mut cursor = io::Cursor::new(wire_packet);
    let parsed = read_handshake_packet(&mut cursor)
        .await
        .expect("failed to parse handshake");

    assert_eq!(parsed.protocol_version, 765);
    assert_eq!(parsed.server_address, "localhost");
    assert_eq!(parsed.server_port, 25565);
    assert_eq!(parsed.next_state, 2);
}

#[test]
fn test_sleeping_status_json() {
    let json_str = make_sleeping_status_json("TestServer", 20, 767);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("invalid json");
    assert_eq!(parsed["players"]["max"], 20);
    assert_eq!(parsed["players"]["online"], 0);
    assert!(parsed["version"]["name"]
        .as_str()
        .unwrap()
        .contains("Sleeping"));
}
