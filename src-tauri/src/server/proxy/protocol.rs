use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Reads a Minecraft VarInt from an async reader
pub async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<i32> {
    let mut result = 0i32;

    for byte_index in 0..5 {
        let byte = reader.read_u8().await?;
        if byte_index == 4 && byte & 0xF0 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "VarInt is too big",
            ));
        }
        let value = i32::from(byte & 0x7F);
        result |= value << (7 * byte_index);
        if (byte & 0x80) == 0 {
            return Ok(result);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "VarInt is too big",
    ))
}

/// Encodes a Minecraft VarInt into a byte buffer
pub fn encode_varint(mut value: i32, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        // Unsigned shift
        value = ((value as u32) >> 7) as i32;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Writes a Minecraft VarInt to an async writer
pub async fn write_varint<W: AsyncWrite + Unpin>(writer: &mut W, value: i32) -> io::Result<()> {
    let mut buf = Vec::with_capacity(5);
    encode_varint(value, &mut buf);
    writer.write_all(&buf).await
}

/// Reads a Minecraft length-prefixed UTF-8 string
pub async fn read_mc_string<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<String> {
    let length = read_varint(reader).await?;
    if !(0..=32767).contains(&length) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "String length out of bounds",
        ));
    }
    let mut buf = vec![0u8; length as usize];
    reader.read_exact(&mut buf).await?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Encodes a Minecraft packet (Packet ID + Payload) with total VarInt length prefix
pub fn encode_packet(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    encode_varint(packet_id, &mut body);
    body.extend_from_slice(payload);

    let mut packet = Vec::new();
    encode_varint(body.len() as i32, &mut packet);
    packet.extend_from_slice(&body);
    packet
}

/// Decoded Minecraft Handshake packet
#[derive(Debug, Clone)]
pub struct HandshakePacket {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32, // 1 = Status, 2 = Login
    pub raw_bytes: Vec<u8>,
}

/// Reads the Handshake packet while preserving all raw bytes for replay
pub async fn read_handshake_packet<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> io::Result<HandshakePacket> {
    let packet_len = read_varint(reader).await?;
    if packet_len <= 0 || packet_len > 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid handshake packet length: {packet_len}"),
        ));
    }

    let mut packet_data = vec![0u8; packet_len as usize];
    reader.read_exact(&mut packet_data).await?;

    // Prepend packet length varint to recreate exact raw wire packet
    let mut raw_bytes = Vec::new();
    encode_varint(packet_len, &mut raw_bytes);
    raw_bytes.extend_from_slice(&packet_data);

    // Parse packet data
    let mut cursor = io::Cursor::new(packet_data);
    let packet_id = read_varint(&mut cursor).await?;
    if packet_id != 0x00 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Expected Handshake packet 0x00, got: {packet_id:#x}"),
        ));
    }

    let protocol_version = read_varint(&mut cursor).await?;
    let server_address = read_mc_string(&mut cursor).await?;
    let server_port = cursor.read_u16().await?;
    let next_state = read_varint(&mut cursor).await?;

    Ok(HandshakePacket {
        protocol_version,
        server_address,
        server_port,
        next_state,
        raw_bytes,
    })
}

/// Creates synthetic status JSON response for sleeping state
pub fn make_sleeping_status_json(server_name: &str, max_players: u32, protocol: i32) -> String {
    let json = serde_json::json!({
        "version": {
            "name": "💤 Sleeping (Auto-Wake)",
            "protocol": if protocol > 0 { protocol } else { 767 }
        },
        "players": {
            "max": max_players,
            "online": 0,
            "sample": []
        },
        "description": {
            "text": format!("§b§l{} §7(Android Host)\n§e💤 Sleeping §7- §fjoin to wake it up!", server_name)
        }
    });
    json.to_string()
}

/// Creates synthetic status JSON response for booting state
pub fn make_booting_status_json(server_name: &str, max_players: u32, protocol: i32) -> String {
    let json = serde_json::json!({
        "version": {
            "name": "⚡ Starting up...",
            "protocol": if protocol > 0 { protocol } else { 767 }
        },
        "players": {
            "max": max_players,
            "online": 0,
            "sample": []
        },
        "description": {
            "text": format!("§b§l{} §7(Android Host)\n§6⚡ Starting up §7- §fretry in a moment", server_name)
        }
    });
    json.to_string()
}

/// Encodes a Minecraft Login Disconnect packet (0x00 in Login phase)
pub fn make_login_disconnect_packet(message: &str) -> Vec<u8> {
    let chat_json = serde_json::json!({ "text": message }).to_string();
    let mut payload = Vec::new();
    encode_varint(chat_json.len() as i32, &mut payload);
    payload.extend_from_slice(chat_json.as_bytes());
    encode_packet(0x00, &payload)
}
