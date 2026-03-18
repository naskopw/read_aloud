mod ffi;
mod voices;

use std::{
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use httpdate::parse_http_date;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tungstenite::{
    client::IntoClientRequest,
    connect,
    error::Error as WsError,
    http::{self, Request},
    Message,
};

pub use ffi::{read_aloud_last_error_message, read_aloud_status_string, text_to_speech, ReadAloudStatus};
pub use voices::Voice;

const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const EDGE_MAJOR_VERSION: &str = "146";
const SEC_MS_GEC_VERSION: &str = "1-146.0.3856.62";
const ORIGIN_VALUE: &str = "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold";
const ACCEPT_LANGUAGE_VALUE: &str = "en-GB,en;q=0.9,en-US;q=0.8";
const ACCEPT_ENCODING_VALUE: &str = "gzip, deflate, br, zstd";
const WIN_EPOCH_SECONDS: f64 = 11_644_473_600.0;

#[derive(Error, Debug)]
pub enum TTSError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("null pointer: {0}")]
    NullPointer(String),
    #[error("invalid UTF-8 in {0}")]
    Utf8(String),
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, TTSError>;

fn websocket_url(connection_id: &str, sec_ms_gec: &str) -> String {
    format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={TRUSTED_CLIENT_TOKEN}&Sec-MS-GEC={sec_ms_gec}&Sec-MS-GEC-Version={SEC_MS_GEC_VERSION}&ConnectionId={connection_id}"
    )
}

fn uid() -> String {
    let id = uuid::Uuid::new_v4().to_string().replace("-", "");
    id
}

fn now_unix_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn date_to_string() -> String {
    httpdate::fmt_http_date(SystemTime::now())
}

fn generate_sec_ms_gec(clock_skew_seconds: f64) -> String {
    let mut ticks = now_unix_seconds() + clock_skew_seconds + WIN_EPOCH_SECONDS;
    ticks -= ticks % 300.0;
    ticks *= 10_000_000.0;

    let input = format!("{ticks:.0}{TRUSTED_CLIENT_TOKEN}");
    let digest = Sha256::digest(input.as_bytes());
    let mut token = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(token, "{byte:02X}");
    }
    token
}

fn user_agent() -> String {
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{EDGE_MAJOR_VERSION}.0.0.0 Safari/537.36 Edg/{EDGE_MAJOR_VERSION}.0.0.0"
    )
}

fn build_websocket_request(clock_skew_seconds: f64) -> Result<Request<()>> {
    let connection_id = uid();
    let request_url = websocket_url(&connection_id, &generate_sec_ms_gec(clock_skew_seconds));
    let mut request = request_url
        .into_client_request()
        .map_err(|error| TTSError::Connection(error.to_string()))?;

    request.headers_mut().insert("Pragma", "no-cache".parse().unwrap());
    request.headers_mut().insert("Cache-Control", "no-cache".parse().unwrap());
    request.headers_mut().insert("User-Agent", user_agent().parse().unwrap());
    request.headers_mut().insert("Origin", ORIGIN_VALUE.parse().unwrap());
    request
        .headers_mut()
        .insert("Accept-Encoding", ACCEPT_ENCODING_VALUE.parse().unwrap());
    request
        .headers_mut()
        .insert("Accept-Language", ACCEPT_LANGUAGE_VALUE.parse().unwrap());
    request.headers_mut().insert(
        "Cookie",
        format!("MUID={};", uid().to_uppercase()).parse().unwrap(),
    );
    request.headers_mut().insert(
        "Sec-MS-GEC-Version",
        SEC_MS_GEC_VERSION.parse().unwrap(),
    );
    request.headers_mut().insert(
        "Sec-WebSocket-Extensions",
        "permessage-deflate; client_max_window_bits".parse().unwrap(),
    );

    Ok(request)
}

fn clock_skew_from_response(response: &http::Response<Option<Vec<u8>>>) -> Option<f64> {
    let date = response.headers().get("Date")?.to_str().ok()?;
    let server_time = parse_http_date(date).ok()?;
    let server_seconds = server_time.duration_since(UNIX_EPOCH).ok()?.as_secs_f64();
    Some(server_seconds - now_unix_seconds())
}

fn open_socket() -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>> {
    let request = build_websocket_request(0.0)?;
    match connect(request) {
        Ok((socket, _)) => Ok(socket),
        Err(WsError::Http(response)) if response.status() == http::StatusCode::FORBIDDEN => {
            let Some(clock_skew_seconds) = clock_skew_from_response(&response) else {
                return Err(TTSError::Connection(format!(
                    "websocket upgrade failed with status {}",
                    response.status()
                )));
            };

            let retry_request = build_websocket_request(clock_skew_seconds)?;
            connect(retry_request)
                .map(|(socket, _)| socket)
                .map_err(|error| TTSError::Connection(error.to_string()))
        }
        Err(error) => Err(TTSError::Connection(error.to_string())),
    }
}

fn setup_request() -> String {
    let body = r#"{"context":{"synthesis":{"audio":{"metadataoptions":{"sentenceBoundaryEnabled":"false","wordBoundaryEnabled":"true"},"outputFormat":"audio-24khz-48kbitrate-mono-mp3"}}}}"#;
    let r = RequestBuilder::new()
        .add_header("X-Timestamp", date_to_string().as_str())
        .add_header("Content-Type", "application/json; charset=utf-8")
        .add_header("Path", "speech.config")
        .build(body);
    r
}

fn tts_request(text: String, voice: Voice, pitch: i32, rate: f32, volume: f32) -> String {
    let pitch = format!("{:+}Hz", pitch);
    let rate = format!("{:+}%", (rate * 100.0).round() as i32);
    let volume = format!("{:+}%", (volume * 100.0).round() as i32);

    let voice: &str = voice.into();
    let body = format!("<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis'  xml:lang='en-US'><voice name='{}'><prosody pitch='{}' rate ='{}' volume='{}'>{}</prosody></voice></speak>", voice, pitch, rate, volume, text);
    let r = RequestBuilder::new()
        .add_header("X-RequestId", uid().as_str())
        .add_header("Content-Type", "application/ssml+xml")
        .add_header("X-Timestamp", format!("{}Z", date_to_string()).as_str())
        .add_header("Path", "ssml")
        .build(body.as_str());
    r
}

fn sanitize_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn parse_binary_response(bin_data: &[u8]) -> Result<Option<&[u8]>> {
    if bin_data.len() < 2 {
        return Err(TTSError::Protocol(
            "binary response missing header length prefix".into(),
        ));
    }

    let header_length = u16::from_be_bytes([bin_data[0], bin_data[1]]) as usize;
    let header_end = 2 + header_length;
    if header_end > bin_data.len() {
        return Err(TTSError::Protocol(
            "binary response header length exceeds payload size".into(),
        ));
    }

    let header_bytes = &bin_data[2..header_end];
    let mut path = None;
    let mut content_type = None;
    for line in header_bytes.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        if let Some(separator_index) = line.iter().position(|byte| *byte == b':') {
            let key = &line[..separator_index];
            let value = &line[separator_index + 1..];
            let value = value.strip_prefix(b" ").unwrap_or(value);
            match key {
                b"Path" => path = Some(value),
                b"Content-Type" => content_type = Some(value),
                _ => {}
            }
        }
    }

    if path != Some(b"audio".as_slice()) {
        return Ok(None);
    }

    if content_type.is_none() && header_end == bin_data.len() {
        return Ok(None);
    }

    Ok(Some(&bin_data[header_end..]))
}

pub fn generate(text: &str, voice: Voice, pitch: i32, rate: f32, volume: f32, f: &Path) -> Result<()> {
    if text.is_empty() {
        return Err(TTSError::InvalidInput("text cannot be empty".into()));
    }
    if rate < -1.0 || rate > 1.0 {
        return Err(TTSError::InvalidInput("rate must be between -1.0 and 1.0".into()));
    }
    if volume < -1.0 || volume > 1.0 {
        return Err(TTSError::InvalidInput("volume must be between -1.0 and 1.0".into()));
    }
    let text = sanitize_text(text);
    let mut socket = open_socket()?;

    let f = std::fs::File::create(f).map_err(|error| TTSError::Io(error.to_string()))?;
    let mut writer = std::io::BufWriter::new(f);

    socket
        .write(Message::Text(setup_request()))
        .map_err(|error| TTSError::Connection(error.to_string()))?;
    socket
        .write(Message::Text(tts_request(text, voice, pitch, rate, volume)))
        .map_err(|error| TTSError::Connection(error.to_string()))?;
    socket
        .flush()
        .map_err(|error| TTSError::Connection(error.to_string()))?;

    loop {
        let msg = socket
            .read()
            .map_err(|error| TTSError::Connection(error.to_string()))?;
        if msg.is_binary() {
            let bin_data = msg.into_data();
            if let Some(audio_data) = parse_binary_response(&bin_data)? {
                writer
                    .write_all(audio_data)
                    .map_err(|error| TTSError::Io(error.to_string()))?;
            }
        } else {
            let string = msg
                .into_text()
                .map_err(|error| TTSError::Protocol(error.to_string()))?;
            let end = string.contains("Path:turn.end");
            if end {
                break;
            }
            // This is good enough for now, as the server will disconnect you after a few seconds after the last response
        }
    }
    Ok(())
}

struct RequestBuilder {
    headers: Vec<String>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self { headers: vec![] }
    }

    pub fn add_header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.push(format!("{}:{}", key, value));
        self
    }

    pub fn build(&self, body: &str) -> String {
        let headers = self.headers.join("\r\n");
        let request = format!("{}\r\n\r\n{}", headers, body);
        request
    }
}
