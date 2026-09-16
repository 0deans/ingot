use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerPlayerSample {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerPlayersInfo {
    pub max: u32,
    pub online: u32,
    pub sample: Option<Vec<ServerPlayerSample>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerVersionInfo {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerPingResponse {
    pub version: ServerVersionInfo,
    pub players: ServerPlayersInfo,
    pub motd: String,
    pub favicon: Option<String>,
    pub ping_ms: u64,
}

fn write_varint(buf: &mut Vec<u8>, mut val: i32) {
    loop {
        if (val & !0x7F) == 0 {
            buf.push(val as u8);
            return;
        }
        buf.push(((val & 0x7F) | 0x80) as u8);
        val = ((val as u32) >> 7) as i32;
    }
}

async fn read_varint<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<i32, String> {
    let mut num_read = 0;
    let mut result = 0i32;
    loop {
        let byte = reader
            .read_u8()
            .await
            .map_err(|e| format!("Failed to read varint byte: {e}"))?;
        let value = (byte & 0x7F) as i32;
        result |= value << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err("VarInt is too big".to_string());
        }
        if (byte & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

fn extract_motd_text(desc_val: &serde_json::Value) -> String {
    if let Some(s) = desc_val.as_str() {
        return s.to_string();
    }
    if let Some(obj) = desc_val.as_object() {
        let mut text = obj.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
        if let Some(extra) = obj.get("extra").and_then(|e| e.as_array()) {
            for item in extra {
                text.push_str(&extract_motd_text(item));
            }
        }
        return text;
    }
    desc_val.to_string()
}

pub async fn ping_server(host: &str, port: u16) -> Result<ServerPingResponse, String> {
    let addr = format!("{}:{}", host, port);
    
    // Connect with a 2-second timeout
    let stream_res = timeout(Duration::from_secs(2), TcpStream::connect(&addr)).await;
    let mut stream = stream_res
        .map_err(|_| format!("Connection timeout to {}", addr))?
        .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

    let start = Instant::now();

    // 1. Construct Handshake Packet (ID: 0x00)
    // Fields: Packet ID (0x00), Protocol Version (VarInt -1 = 47 or 765), Server Address (String), Server Port (u16), Next State (VarInt 1 = status)
    let mut handshake_data = Vec::new();
    write_varint(&mut handshake_data, 0x00); // packet id 0
    write_varint(&mut handshake_data, 765);  // protocol version (1.20+)
    
    // Server host string
    write_varint(&mut handshake_data, host.len() as i32);
    handshake_data.extend_from_slice(host.as_bytes());
    
    // Server port (u16 be)
    handshake_data.extend_from_slice(&port.to_be_bytes());
    
    // Next state: 1 (status)
    write_varint(&mut handshake_data, 1);

    // Frame handshake packet with length
    let mut handshake_packet = Vec::new();
    write_varint(&mut handshake_packet, handshake_data.len() as i32);
    handshake_packet.extend_from_slice(&handshake_data);

    // 2. Construct Status Request Packet (ID: 0x00, empty body)
    let mut request_packet = Vec::new();
    write_varint(&mut request_packet, 1); // length = 1
    write_varint(&mut request_packet, 0x00); // packet ID = 0x00

    // Send Handshake + Request together
    stream
        .write_all(&handshake_packet)
        .await
        .map_err(|e| format!("Failed to send handshake: {e}"))?;
    stream
        .write_all(&request_packet)
        .await
        .map_err(|e| format!("Failed to send status request: {e}"))?;

    // 3. Read Response Packet
    let _packet_len = read_varint(&mut stream).await?;
    let packet_id = read_varint(&mut stream).await?;
    if packet_id != 0x00 {
        return Err(format!("Unexpected packet ID from server: {}", packet_id));
    }

    let json_len = read_varint(&mut stream).await? as usize;
    let mut json_bytes = vec![0u8; json_len];
    stream
        .read_exact(&mut json_bytes)
        .await
        .map_err(|e| format!("Failed to read response JSON: {e}"))?;

    let ping_ms = start.elapsed().as_millis() as u64;

    let raw_str = String::from_utf8(json_bytes)
        .map_err(|e| format!("Invalid UTF-8 in status response: {e}"))?;

    let json_val: serde_json::Value = serde_json::from_str(&raw_str)
        .map_err(|e| format!("Failed to parse status JSON: {e}"))?;

    let version_name = json_val
        .get("version")
        .and_then(|v| v.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("Unknown")
        .to_string();
    let protocol = json_val
        .get("version")
        .and_then(|v| v.get("protocol"))
        .and_then(|p| p.as_i64())
        .unwrap_or(0) as i32;

    let max_players = json_val
        .get("players")
        .and_then(|p| p.get("max"))
        .and_then(|m| m.as_u64())
        .unwrap_or(0) as u32;
    let online_players = json_val
        .get("players")
        .and_then(|p| p.get("online"))
        .and_then(|o| o.as_u64())
        .unwrap_or(0) as u32;

    let sample = json_val
        .get("players")
        .and_then(|p| p.get("sample"))
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    let id = item.get("id")?.as_str()?.to_string();
                    Some(ServerPlayerSample { name, id })
                })
                .collect()
        });

    let motd = json_val
        .get("description")
        .map(extract_motd_text)
        .unwrap_or_default();

    let favicon = json_val
        .get("favicon")
        .and_then(|f| f.as_str())
        .map(|s| s.to_string());

    Ok(ServerPingResponse {
        version: ServerVersionInfo {
            name: version_name,
            protocol,
        },
        players: ServerPlayersInfo {
            max: max_players,
            online: online_players,
            sample,
        },
        motd,
        favicon,
        ping_ms,
    })
}
