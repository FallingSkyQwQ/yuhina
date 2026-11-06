use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::paths::Paths;
use super::storage::{read_json, write_json};
use super::{ErrorCode, Result, YuhinaError};

pub const PING_TIMEOUT_MS: u64 = 8_000;
const PROTOCOL_VERSION: i32 = 767;
const MAX_PACKET_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub last_played_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ServersFile {
    pub version: u32,
    pub servers: Vec<ServerRecord>,
}

impl Default for ServersFile {
    fn default() -> Self {
        Self {
            version: 1,
            servers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ServerStatus {
    pub version_name: String,
    pub protocol: i32,
    pub players_online: u32,
    pub players_max: u32,
    pub sample: Vec<String>,
    pub motd: String,
    pub favicon: Option<String>,
    pub latency_ms: u64,
}

pub fn list(paths: &Paths) -> Result<Vec<ServerRecord>> {
    Ok(read_json::<ServersFile>(&paths.servers_file())?
        .unwrap_or_default()
        .servers)
}

pub fn save(paths: &Paths, server: ServerRecord) -> Result<Vec<ServerRecord>> {
    let mut file = read_json::<ServersFile>(&paths.servers_file())?.unwrap_or_default();
    match file
        .servers
        .iter_mut()
        .find(|candidate| candidate.id == server.id)
    {
        Some(existing) => *existing = server,
        None => file.servers.push(server),
    }
    write_json(&paths.servers_file(), &file)?;
    Ok(file.servers)
}

pub fn add(
    paths: &Paths,
    name: &str,
    address: &str,
    instance_id: Option<String>,
) -> Result<ServerRecord> {
    let (host, port) = parse_address(address)?;
    let server = ServerRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: if name.trim().is_empty() {
            host.clone()
        } else {
            name.trim().to_string()
        },
        address: host,
        port,
        icon: None,
        instance_id,
        note: String::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_played_at: None,
    };
    save(paths, server.clone())?;
    Ok(server)
}

pub fn remove(paths: &Paths, id: &str) -> Result<Vec<ServerRecord>> {
    let mut file = read_json::<ServersFile>(&paths.servers_file())?.unwrap_or_default();
    file.servers.retain(|server| server.id != id);
    write_json(&paths.servers_file(), &file)?;
    Ok(file.servers)
}

/// Accepts `host`, `host:port`, `[::1]:port`.
pub fn parse_address(input: &str) -> Result<(String, u16)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "Enter a server address.",
        ));
    }
    if let Some(rest) = trimmed.strip_prefix('[')
        && let Some((host, tail)) = rest.split_once(']')
    {
        let port = tail
            .strip_prefix(':')
            .and_then(|port| port.parse().ok())
            .unwrap_or(25565);
        return Ok((host.to_string(), port));
    }
    match trimmed.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') && !port.is_empty() => {
            let port = port.parse::<u16>().map_err(|_| {
                YuhinaError::new(ErrorCode::InvalidInput, "That server port is invalid.")
            })?;
            Ok((host.to_string(), port))
        }
        _ => Ok((trimmed.to_string(), 25565)),
    }
}

pub fn format_address(host: &str, port: u16) -> String {
    if port == 25565 {
        host.to_string()
    } else if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub fn ping(host: &str, port: u16) -> Result<ServerStatus> {
    let started = Instant::now();
    let address = (host, port).to_socket_addrs().map_err(|error| {
        YuhinaError::new(
            ErrorCode::Network,
            format!("The server host could not be resolved: {error}"),
        )
    })?;
    let mut last_error = None;
    let mut stream = None;
    for addr in address {
        match TcpStream::connect_timeout(&addr, Duration::from_millis(PING_TIMEOUT_MS)) {
            Ok(connected) => {
                stream = Some(connected);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let mut stream = stream.ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::Network,
            format!(
                "The server could not be reached: {}",
                last_error
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "no address".to_string())
            ),
        )
    })?;
    stream.set_read_timeout(Some(Duration::from_millis(PING_TIMEOUT_MS)))?;
    stream.set_write_timeout(Some(Duration::from_millis(PING_TIMEOUT_MS)))?;

    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0x00);
    write_varint(&mut handshake, PROTOCOL_VERSION);
    write_string(&mut handshake, host);
    handshake.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake, 1);
    let mut packet = Vec::new();
    write_varint(&mut packet, handshake.len() as i32);
    packet.extend_from_slice(&handshake);
    write_varint(&mut packet, 1);
    write_varint(&mut packet, 0x00);
    stream.write_all(&packet)?;
    stream.flush()?;

    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let deadline = Instant::now() + Duration::from_millis(PING_TIMEOUT_MS);
    loop {
        if Instant::now() > deadline {
            return Err(YuhinaError::new(
                ErrorCode::Network,
                "The server did not respond in time.",
            ));
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_PACKET_BYTES {
            return Err(YuhinaError::new(
                ErrorCode::Network,
                "The server response was too large.",
            ));
        }
        if let Some(json) = parse_status_response(&buffer)? {
            let latency_ms = started.elapsed().as_millis() as u64;
            return parse_status_json(&json, latency_ms);
        }
    }
    Err(YuhinaError::new(
        ErrorCode::Network,
        "The server closed the connection.",
    ))
}

fn parse_status_response(buffer: &[u8]) -> Result<Option<String>> {
    let Some((length, length_size)) = read_varint(buffer, 0) else {
        return Ok(None);
    };
    let length = length as usize;
    if length > MAX_PACKET_BYTES {
        return Err(YuhinaError::new(
            ErrorCode::Network,
            "The server response was too large.",
        ));
    }
    if buffer.len() < length_size + length {
        return Ok(None);
    }
    let body = &buffer[length_size..length_size + length];
    let Some((packet_id, id_size)) = read_varint(body, 0) else {
        return Err(YuhinaError::new(
            ErrorCode::Network,
            "The server returned a malformed packet.",
        ));
    };
    if packet_id != 0 {
        return Err(YuhinaError::new(
            ErrorCode::Network,
            "The server returned an unexpected packet.",
        ));
    }
    let Some((string_length, string_size)) = read_varint(body, id_size) else {
        return Err(YuhinaError::new(
            ErrorCode::Network,
            "The server returned a malformed packet.",
        ));
    };
    let start = id_size + string_size;
    let end = start + string_length as usize;
    if end > body.len() {
        return Err(YuhinaError::new(
            ErrorCode::Network,
            "The server returned a malformed packet.",
        ));
    }
    Ok(Some(String::from_utf8_lossy(&body[start..end]).to_string()))
}

fn parse_status_json(json: &str, latency_ms: u64) -> Result<ServerStatus> {
    let value: Value = serde_json::from_str(json).map_err(|error| {
        YuhinaError::new(
            ErrorCode::Network,
            format!("The server returned invalid status JSON: {error}"),
        )
    })?;
    let sample = value["players"]["sample"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(ServerStatus {
        version_name: value["version"]["name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string(),
        protocol: value["version"]["protocol"].as_i64().unwrap_or(0) as i32,
        players_online: value["players"]["online"].as_u64().unwrap_or(0) as u32,
        players_max: value["players"]["max"].as_u64().unwrap_or(0) as u32,
        sample,
        motd: flatten_description(&value["description"]),
        favicon: value["favicon"].as_str().map(str::to_string),
        latency_ms,
    })
}

fn flatten_description(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    let mut result = String::new();
    if let Some(text) = value["text"].as_str() {
        result.push_str(text);
    }
    if let Some(extra) = value["extra"].as_array() {
        for part in extra {
            result.push_str(&flatten_description(part));
        }
    }
    if result.is_empty() {
        "Minecraft server".to_string()
    } else {
        result
    }
}

fn write_varint(out: &mut Vec<u8>, value: i32) {
    let mut value = value as u32;
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_varint(out, value.len() as i32);
    out.extend_from_slice(value.as_bytes());
}

fn read_varint(buffer: &[u8], offset: usize) -> Option<(i32, usize)> {
    let mut value = 0i32;
    let mut shift = 0;
    for (index, &byte) in buffer.iter().enumerate().skip(offset).take(5) {
        value |= ((byte & 0x7f) as i32) << shift;
        if byte & 0x80 == 0 {
            return Some((value, index - offset + 1));
        }
        shift += 7;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_parse() {
        assert_eq!(
            parse_address("play.example.com").unwrap(),
            ("play.example.com".into(), 25565)
        );
        assert_eq!(
            parse_address("play.example.com:25566").unwrap(),
            ("play.example.com".into(), 25566)
        );
        assert_eq!(parse_address("[::1]:25567").unwrap(), ("::1".into(), 25567));
        assert!(parse_address("  ").is_err());
    }

    #[test]
    fn addresses_format() {
        assert_eq!(
            format_address("play.example.com", 25565),
            "play.example.com"
        );
        assert_eq!(
            format_address("play.example.com", 25566),
            "play.example.com:25566"
        );
        assert_eq!(format_address("::1", 25566), "[::1]:25566");
    }
}
