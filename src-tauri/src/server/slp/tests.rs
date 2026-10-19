use super::*;
use std::error::Error;
use tokio::net::TcpListener;

#[tokio::test]
async fn malformed_utf8_response_preserves_decoding_cause() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        for _ in 0..2 {
            let packet_length = read_varint(&mut stream).await.unwrap();
            let mut packet = vec![0; usize::try_from(packet_length).unwrap()];
            stream.read_exact(&mut packet).await.unwrap();
        }
        stream.write_all(&[3, 0, 1, 0xff]).await.unwrap();
    });

    let error = timeout(Duration::from_secs(2), ping_server("127.0.0.1", port))
        .await
        .unwrap()
        .unwrap_err();
    server.await.unwrap();

    assert!(matches!(error, PingError::Utf8(_)));
    let cause = error.source().unwrap();
    let utf8 = cause.downcast_ref::<std::string::FromUtf8Error>().unwrap();
    assert_eq!(utf8.as_bytes(), &[0xff]);
}

#[tokio::test]
async fn oversized_varint_preserves_invalid_data_cause() {
    let bytes = [0x80; 6];
    let mut reader = bytes.as_slice();

    let error = read_varint(&mut reader).await.unwrap_err();

    assert!(matches!(error, PingError::InvalidVarInt(_)));
    let cause = error.source().unwrap();
    let io = cause.downcast_ref::<std::io::Error>().unwrap();
    assert_eq!(io.kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn truncated_varint_remains_network_io_failure() {
    let mut reader = [0x80].as_slice();

    let error = read_varint(&mut reader).await.unwrap_err();

    assert!(matches!(error, PingError::Io(_)));
    let cause = error.source().unwrap();
    let io = cause.downcast_ref::<std::io::Error>().unwrap();
    assert_eq!(io.kind(), std::io::ErrorKind::UnexpectedEof);
}
