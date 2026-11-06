use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use hickory_resolver::proto::rr::RData;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::paths::Paths;
use super::storage::{read_json, write_json};
use super::{ErrorCode, Result, YuhinaError};

pub const PING_TIMEOUT_MS: u64 = 8_000;
const PROTOCOL_VERSION: i32 = 767;
const MAX_PACKET_BYTES: usize = 1024 * 1024;

/// The address to open a TCP connection to, plus the hostname that goes into
/// the Minecraft handshake. They differ when an SRV record redirects traffic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerEndpoint {
    pub connect_host: String,
    pub connect_port: u16,
    pub handshake_host: String,
    pub handshake_port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SrvRecord {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
}

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

/// The SRV query name for a Minecraft Java server, or `None` when the host is
/// already an IP literal.
pub fn srv_lookup_name(host: &str) -> Option<String> {
    let host = host.trim().trim_end_matches('.');
    if host.is_empty() || host.parse::<IpAddr>().is_ok() {
        return None;
    }
    Some(format!("_minecraft._tcp.{host}."))
}

/// Only an implicit/default port is eligible for SRV: an explicitly entered
/// `host:port` is what the player asked for.
pub fn should_lookup_srv(host: &str, port: u16) -> bool {
    port == 25565 && srv_lookup_name(host).is_some()
}

/// Picks the RFC 2782 target: the lowest priority, then the highest weight,
/// then a stable lexical tie-break. A target of `.` means the service is not
/// offered and is ignored.
pub fn select_srv(records: &[SrvRecord]) -> Option<SrvRecord> {
    records
        .iter()
        .filter(|record| !record.target.is_empty() && record.target != ".")
        .min_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| b.weight.cmp(&a.weight))
                .then_with(|| a.target.cmp(&b.target))
                .then_with(|| a.port.cmp(&b.port))
        })
        .cloned()
}

fn endpoint_for(host: &str, port: u16, record: Option<SrvRecord>) -> ServerEndpoint {
    let handshake_host = host.trim().trim_end_matches('.').to_string();
    match record {
        Some(record) => ServerEndpoint {
            connect_host: record.target.trim_end_matches('.').to_string(),
            connect_port: record.port,
            handshake_host,
            handshake_port: port,
        },
        None => ServerEndpoint {
            connect_host: handshake_host.clone(),
            connect_port: port,
            handshake_host,
            handshake_port: port,
        },
    }
}

/// Resolves `_minecraft._tcp.<host>` and falls back to the literal host:port
/// on NXDOMAIN, no SRV record, or any resolver failure.
pub fn resolve_endpoint(host: &str, port: u16) -> ServerEndpoint {
    let record = if should_lookup_srv(host, port) {
        lookup_srv(host).unwrap_or(None)
    } else {
        None
    };
    endpoint_for(host, port, record)
}

fn lookup_srv(host: &str) -> Result<Option<SrvRecord>> {
    let Some(name) = srv_lookup_name(host) else {
        return Ok(None);
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            YuhinaError::new(
                ErrorCode::Network,
                format!("The DNS runtime could not start: {error}"),
            )
        })?;
    let result = runtime.block_on(async move {
        let resolver = hickory_resolver::Resolver::builder_tokio()
            .map_err(|error| YuhinaError::new(ErrorCode::Network, error.to_string()))?
            .build()
            .map_err(|error| YuhinaError::new(ErrorCode::Network, error.to_string()))?;
        let lookup = resolver
            .srv_lookup(name)
            .await
            .map_err(|error| YuhinaError::new(ErrorCode::Network, error.to_string()))?;
        let mut records = Vec::new();
        for record in lookup.answers() {
            if let RData::SRV(srv) = &record.data {
                records.push(SrvRecord {
                    priority: srv.priority,
                    weight: srv.weight,
                    port: srv.port,
                    target: srv.target.to_utf8(),
                });
            }
        }
        Ok::<_, YuhinaError>(records)
    });
    match result {
        Ok(records) => Ok(select_srv(&records)),
        // NXDOMAIN, no SRV records and transient resolver failures all mean
        // "use the literal address".
        Err(_) => Ok(None),
    }
}

pub fn ping(host: &str, port: u16) -> Result<ServerStatus> {
    let started = Instant::now();
    let endpoint = resolve_endpoint(host, port);
    let address = (endpoint.connect_host.as_str(), endpoint.connect_port)
        .to_socket_addrs()
        .map_err(|error| {
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
    write_string(&mut handshake, &endpoint.handshake_host);
    handshake.extend_from_slice(&endpoint.handshake_port.to_be_bytes());
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

    #[test]
    fn srv_names_are_built_only_for_hostnames() {
        assert_eq!(
            srv_lookup_name("play.example.com").as_deref(),
            Some("_minecraft._tcp.play.example.com.")
        );
        assert_eq!(
            srv_lookup_name(" play.example.com. ").as_deref(),
            Some("_minecraft._tcp.play.example.com.")
        );
        assert_eq!(srv_lookup_name("127.0.0.1"), None);
        assert_eq!(srv_lookup_name("::1"), None);
        assert_eq!(srv_lookup_name(""), None);
    }

    #[test]
    fn srv_is_only_used_for_the_default_port() {
        assert!(should_lookup_srv("play.example.com", 25565));
        assert!(!should_lookup_srv("play.example.com", 25566));
        assert!(!should_lookup_srv("127.0.0.1", 25565));
    }

    #[test]
    fn srv_selection_prefers_priority_then_weight() {
        let records = vec![
            SrvRecord {
                priority: 10,
                weight: 1,
                port: 25566,
                target: "low.example.com".into(),
            },
            SrvRecord {
                priority: 5,
                weight: 1,
                port: 25567,
                target: "busy.example.com".into(),
            },
            SrvRecord {
                priority: 5,
                weight: 20,
                port: 25568,
                target: "quiet.example.com".into(),
            },
            SrvRecord {
                priority: 1,
                weight: 999,
                port: 25569,
                target: ".".into(),
            },
        ];
        let selected = select_srv(&records).unwrap();
        assert_eq!(selected.target, "quiet.example.com");
        assert_eq!(selected.port, 25568);
        assert!(select_srv(&[]).is_none());
    }

    #[test]
    fn literal_address_is_the_fallback() {
        let literal = endpoint_for("play.example.com", 25565, None);
        assert_eq!(
            literal,
            ServerEndpoint {
                connect_host: "play.example.com".into(),
                connect_port: 25565,
                handshake_host: "play.example.com".into(),
                handshake_port: 25565,
            }
        );

        let redirected = endpoint_for(
            "play.example.com",
            25565,
            Some(SrvRecord {
                priority: 0,
                weight: 0,
                port: 25570,
                target: "node.example.net.".into(),
            }),
        );
        assert_eq!(redirected.connect_host, "node.example.net");
        assert_eq!(redirected.connect_port, 25570);
        assert_eq!(redirected.handshake_host, "play.example.com");
        assert_eq!(redirected.handshake_port, 25565);
    }
}
