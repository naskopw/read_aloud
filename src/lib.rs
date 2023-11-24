mod ffi;
mod voices;

use std::{io::Write, path::Path};
use thiserror::Error;
use tungstenite::connect;
use url::Url;

pub use ffi::text_to_speech;
pub use voices::Voice;

// TODO: Add proper error handling

#[derive(Error, Debug)]
pub enum TTSError {
    #[error("unknown error")]
    Unknown,
}

impl From<tungstenite::Error> for TTSError {
    fn from(_error: tungstenite::Error) -> Self {
        TTSError::Unknown
    }
}

impl From<std::io::Error> for TTSError {
    fn from(_error: std::io::Error) -> Self {
        TTSError::Unknown
    }
}

pub type Result<T> = std::result::Result<T, TTSError>;

fn build_url() -> Url {
    const TRUSTED_CLIENT_TOKEN: &'static str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
    let id = uid();
    let endpoint = Url::parse(&format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={}&ConnectionId={}",
        TRUSTED_CLIENT_TOKEN, id)).expect("failed to parse service endpoint");
    endpoint
}

fn uid() -> String {
    let id = uuid::Uuid::new_v4().to_string().replace("-", "");
    id
}

fn setup_request() -> String {
    let body = r#"{"context":{"synthesis":{"audio":{"metadataoptions":{"sentenceBoundaryEnabled":"false","wordBoundaryEnabled":"true"},"outputFormat":"audio-24khz-48kbitrate-mono-mp3"}}}}"#;
    let r = RequestBuilder::new()
        .add_header("X-Timestamp", "0")
        .add_header("Content-Type", "application/json; charset=utf-8")
        .add_header("Path", "speech.config")
        .build(body);
    r
}

fn tts_request(text: String, voice: Voice) -> String {
    let voice: &str = voice.into();
    let body = format!("<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis'  xml:lang='en-US'><voice name='{}'><prosody pitch='+0Hz' rate ='+0%' volume='+0%'>{}</prosody></voice></speak>", voice, text);
    let r = RequestBuilder::new()
        .add_header("X-RequestId", uid().as_str())
        .add_header("Content-Type", "application/ssml+xml")
        .add_header("X-Timestamp", "0")
        .add_header("Path", "ssml")
        .build(body.as_str());
    r
}

fn sanitize_text(text: &str) -> String {
    text.replace("<", "").replace(">", "")
}

pub fn generate(text: &str, voice: Voice, f: &Path) -> Result<()> {
    let text = sanitize_text(text);
    let (mut socket, _) = connect(build_url())?;

    let f = std::fs::File::create(f)?;
    let mut writer = std::io::BufWriter::new(f);

    socket.write(setup_request().into())?;
    socket.write(tts_request(text.into(), voice).into())?;
    socket.flush()?;

    loop {
        let msg = socket.read().expect("Error reading message");
        if msg.is_binary() {
            let bin_data = msg.into_data();
            let str_data = String::from_utf8_lossy(&bin_data).to_string();
            let split_index = str_data
                .find("Path:audio")
                .expect("missing header in response Path:audio")
                + 10;
            let audio_data = &bin_data[split_index..];
            writer.write_all(audio_data)?;
        } else {
            let string = msg.into_text()?;
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
