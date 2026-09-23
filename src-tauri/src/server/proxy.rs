use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

/// Reads a Minecraft VarInt from an async reader
pub async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<i32> {
    let mut num_read = 0;
    let mut result = 0i32;

    loop {
        let byte = reader.read_u8().await?;
        let value = (byte & 0x7F) as i32;
        result |= value << (7 * num_read);

        num_read += 1;
        if num_read > 5 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "VarInt is too big",
            ));
        }

        if (byte & 0x80) == 0 {
            break;
        }
    }

    Ok(result)
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
    if length < 0 || length > 32767 {
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
            "text": format!("§b§l{} §7(Android Host)\n§e💤 Server is sleeping to save battery. Connect to wake!", server_name)
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
            "text": format!("§b§l{} §7(Android Host)\n§6⚡ Server is booting up! Please wait...", server_name)
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

/// Shared proxy state configuration
#[derive(Clone)]
pub struct ProxyConfig {
    pub server_id: String,
    pub server_name: String,
    pub public_port: u16,
    pub internal_port: u16,
    pub max_players: u32,
    pub sleep_enabled: bool,
}

/// Handle a single incoming client connection to the proxy
pub async fn handle_proxy_client<FWake, FIsRunning, FIsSleeping>(
    mut client_stream: TcpStream,
    config: ProxyConfig,
    active_conns: Arc<AtomicUsize>,
    wake_fn: Arc<FWake>,
    is_running_fn: Arc<FIsRunning>,
    is_sleeping_fn: Arc<FIsSleeping>,
) -> io::Result<()>
where
    FWake: Fn() -> tokio::sync::oneshot::Receiver<bool> + Send + Sync + 'static,
    FIsRunning: Fn() -> bool + Send + Sync + 'static,
    FIsSleeping: Fn() -> bool + Send + Sync + 'static,
{
    // Read the Minecraft Handshake packet
    let handshake = read_handshake_packet(&mut client_stream).await?;

    match handshake.next_state {
        1 => {
            // NextState = 1: Status (MOTD Server List Ping)
            if is_sleeping_fn() {
                // Server is sleeping -> Respond with synthetic MOTD immediately without waking server
                handle_synthetic_status_query(
                    &mut client_stream,
                    &config.server_name,
                    config.max_players,
                    handshake.protocol_version,
                )
                .await?;
            } else if !is_running_fn() {
                // Server is booting up -> Respond with booting MOTD
                handle_synthetic_booting_status(
                    &mut client_stream,
                    &config.server_name,
                    config.max_players,
                    handshake.protocol_version,
                )
                .await?;
            } else {
                // Server is running -> Forward ping to internal port
                if let Ok(mut server_stream) =
                    TcpStream::connect(format!("127.0.0.1:{}", config.internal_port)).await
                {
                    server_stream.write_all(&handshake.raw_bytes).await?;
                    let _ = tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await;
                }
            }
        }
        2 => {
            // NextState = 2: Login (Player connecting to play)
            if is_sleeping_fn() {
                // Trigger wake up and hold the client socket open!
                let wake_rx = wake_fn();

                // Wait for wake signal or timeout (max 25 seconds before client timeout)
                let wake_result = tokio::time::timeout(Duration::from_secs(25), wake_rx).await;

                let is_ready = match wake_result {
                    Ok(Ok(true)) => true,
                    _ => false,
                };

                if is_ready {
                    // Try connecting to internal port
                    if let Ok(mut server_stream) =
                        TcpStream::connect(format!("127.0.0.1:{}", config.internal_port)).await
                    {
                        server_stream.write_all(&handshake.raw_bytes).await?;
                        active_conns.fetch_add(1, Ordering::SeqCst);
                        let _ =
                            tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await;
                        active_conns.fetch_sub(1, Ordering::SeqCst);
                        return Ok(());
                    }
                }

                // If wake timed out or connection failed, send friendly disconnect notice
                let disconnect_pkt = make_login_disconnect_packet(
                    "§6[Ingot Mobile] §eServer is waking up from sleep!\n§aPlease reconnect in 10 seconds.",
                );
                let _ = client_stream.write_all(&disconnect_pkt).await;
                let _ = client_stream.flush().await;
            } else if is_running_fn() {
                // Server is already running -> Connect immediately and splice
                let mut server_stream =
                    TcpStream::connect(format!("127.0.0.1:{}", config.internal_port))
                        .await
                        .map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::ConnectionRefused,
                                format!("Could not connect to internal server port {}: {e}", config.internal_port),
                            )
                        })?;

                server_stream.write_all(&handshake.raw_bytes).await?;
                active_conns.fetch_add(1, Ordering::SeqCst);
                let _ = tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await;
                active_conns.fetch_sub(1, Ordering::SeqCst);
            } else {
                // Server is currently starting up: wait up to 20s for internal port
                for _ in 0..40 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    if let Ok(mut server_stream) =
                        TcpStream::connect(format!("127.0.0.1:{}", config.internal_port)).await
                    {
                        server_stream.write_all(&handshake.raw_bytes).await?;
                        active_conns.fetch_add(1, Ordering::SeqCst);
                        let _ =
                            tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await;
                        active_conns.fetch_sub(1, Ordering::SeqCst);
                        return Ok(());
                    }
                }

                let disconnect_pkt = make_login_disconnect_packet(
                    "§6[Ingot Mobile] §eServer is still starting up. Please reconnect in a moment!",
                );
                let _ = client_stream.write_all(&disconnect_pkt).await;
                let _ = client_stream.flush().await;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Answers the 2-step status handshake (0x00 request + 0x01 ping) with synthetic MOTD
async fn handle_synthetic_status_query<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    server_name: &str,
    max_players: u32,
    protocol: i32,
) -> io::Result<()> {
    // 1. Read Status Request (Length: 1, Packet ID: 0x00)
    let _req_len = read_varint(stream).await?;
    let req_id = read_varint(stream).await?;
    if req_id != 0x00 {
        return Ok(());
    }

    // 2. Send Status Response (Packet ID: 0x00, JSON payload)
    let json_resp = make_sleeping_status_json(server_name, max_players, protocol);
    let mut payload = Vec::new();
    encode_varint(json_resp.len() as i32, &mut payload);
    payload.extend_from_slice(json_resp.as_bytes());

    let packet = encode_packet(0x00, &payload);
    stream.write_all(&packet).await?;
    stream.flush().await?;

    // 3. Read Ping Request (Length: 9, Packet ID: 0x01, 8-byte payload)
    if let Ok(_ping_len) = read_varint(stream).await {
        if let Ok(ping_id) = read_varint(stream).await {
            if ping_id == 0x01 {
                let mut ping_payload = [0u8; 8];
                if stream.read_exact(&mut ping_payload).await.is_ok() {
                    // 4. Send Pong Response (Packet ID: 0x01, identical 8-byte payload)
                    let pong_packet = encode_packet(0x01, &ping_payload);
                    let _ = stream.write_all(&pong_packet).await;
                    let _ = stream.flush().await;
                }
            }
        }
    }

    Ok(())
}

/// Answers status query during boot phase
async fn handle_synthetic_booting_status<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    server_name: &str,
    max_players: u32,
    protocol: i32,
) -> io::Result<()> {
    let _req_len = read_varint(stream).await?;
    let req_id = read_varint(stream).await?;
    if req_id != 0x00 {
        return Ok(());
    }

    let json_resp = make_booting_status_json(server_name, max_players, protocol);
    let mut payload = Vec::new();
    encode_varint(json_resp.len() as i32, &mut payload);
    payload.extend_from_slice(json_resp.as_bytes());

    let packet = encode_packet(0x00, &payload);
    stream.write_all(&packet).await?;
    stream.flush().await?;

    if let Ok(_ping_len) = read_varint(stream).await {
        if let Ok(ping_id) = read_varint(stream).await {
            if ping_id == 0x01 {
                let mut ping_payload = [0u8; 8];
                if stream.read_exact(&mut ping_payload).await.is_ok() {
                    let pong_packet = encode_packet(0x01, &ping_payload);
                    let _ = stream.write_all(&pong_packet).await;
                    let _ = stream.flush().await;
                }
            }
        }
    }

    Ok(())
}

/// Manages a running TCP proxy instance
pub struct ServerProxyHandle {
    pub shutdown_tx: broadcast::Sender<()>,
    pub active_connections: Arc<AtomicUsize>,
    pub is_running: Arc<AtomicBool>,
}

/// Spawns the TCP proxy listener on public_port
pub async fn start_server_proxy<FWake, FIsRunning, FIsSleeping>(
    config: ProxyConfig,
    wake_fn: FWake,
    is_running_fn: FIsRunning,
    is_sleeping_fn: FIsSleeping,
) -> io::Result<ServerProxyHandle>
where
    FWake: Fn() -> tokio::sync::oneshot::Receiver<bool> + Send + Sync + 'static,
    FIsRunning: Fn() -> bool + Send + Sync + 'static,
    FIsSleeping: Fn() -> bool + Send + Sync + 'static,
{
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.public_port)).await?;
    let (shutdown_tx, _) = broadcast::channel(1);
    let active_connections = Arc::new(AtomicUsize::new(0));
    let is_running = Arc::new(AtomicBool::new(true));

    let mut shutdown_rx = shutdown_tx.subscribe();
    let conns_clone = active_connections.clone();
    let is_running_clone = is_running.clone();
    let wake_arc = Arc::new(wake_fn);
    let is_running_arc = Arc::new(is_running_fn);
    let is_sleeping_arc = Arc::new(is_sleeping_fn);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, _addr)) => {
                            let cfg = config.clone();
                            let conns = conns_clone.clone();
                            let wake = wake_arc.clone();
                            let running = is_running_arc.clone();
                            let sleeping = is_sleeping_arc.clone();

                            tokio::spawn(async move {
                                let _ = handle_proxy_client(
                                    stream,
                                    cfg,
                                    conns,
                                    wake,
                                    running,
                                    sleeping,
                                ).await;
                            });
                        }
                        Err(e) => {
                            eprintln!("[Proxy] Accept error: {e}");
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    is_running_clone.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }
    });

    Ok(ServerProxyHandle {
        shutdown_tx,
        active_connections,
        is_running,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_varint_roundtrip() {
        let test_values = vec![0, 1, 2, 127, 128, 255, 256, 2147483647, -1];
        for val in test_values {
            let mut buf = Vec::new();
            encode_varint(val, &mut buf);

            let mut cursor = io::Cursor::new(buf);
            let decoded = read_varint(&mut cursor).await.expect("failed to decode varint");
            assert_eq!(val, decoded);
        }
    }

    #[tokio::test]
    async fn test_handshake_decode() {
        // Construct a synthetic Handshake packet
        // Handshake: Packet ID 0x00, Protocol: 765, Address: "localhost", Port: 25565, NextState: 2
        let mut body = Vec::new();
        encode_varint(0x00, &mut body); // packet id
        encode_varint(765, &mut body); // protocol version
        let addr = "localhost";
        encode_varint(addr.len() as i32, &mut body);
        body.extend_from_slice(addr.as_bytes());
        body.extend_from_slice(&25565u16.to_be_bytes());
        encode_varint(2, &mut body); // NextState = 2 (Login)

        let mut wire_packet = Vec::new();
        encode_varint(body.len() as i32, &mut wire_packet);
        wire_packet.extend_from_slice(&body);

        let mut cursor = io::Cursor::new(wire_packet);
        let parsed = read_handshake_packet(&mut cursor).await.expect("failed to parse handshake");

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
        assert!(parsed["version"]["name"].as_str().unwrap().contains("Sleeping"));
    }
}
