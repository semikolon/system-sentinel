use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use serde::Serialize;
use tracing::{debug, warn};

const SOCKET_PATH: &str = "/tmp/claude-tts-daemon.sock";

#[derive(Serialize)]
struct NarrationRequest {
    #[serde(rename = "type")]
    request_type: String,
    text: String,
    voice: String,
}

pub struct Narrator {
    socket_path: String,
}

impl Narrator {
    pub fn new() -> Self {
        Self {
            socket_path: SOCKET_PATH.to_string(),
        }
    }

    /// Send a narration request to the background daemon.
    /// This is non-blocking on the audio playback itself, 
    /// but blocking on the socket communication (which is instant).
    pub fn narrate(&self, text: &str) -> Result<()> {
        if !Path::new(&self.socket_path).exists() {
            debug!("TTS daemon socket not found at {}, skipping narration", self.socket_path);
            return Ok(());
        }

        let request = NarrationRequest {
            request_type: "arbitrary_narration".to_string(),
            text: text.to_string(),
            voice: "lucy".to_string(), // Matches user's preferred voice in config
        };

        let payload = serde_json::to_vec(&request)?;
        let length_prefix = (payload.len() as u32).to_be_bytes();

        debug!("Connecting to TTS daemon at {}", self.socket_path);
        let mut stream = UnixStream::connect(&self.socket_path)
            .context("Failed to connect to TTS daemon socket")?;

        // Format is [4 bytes length][JSON payload]
        stream.write_all(&length_prefix)?;
        stream.write_all(&payload)?;
        stream.flush()?;

        // Wait for OK response
        let mut response = [0u8; 2];
        let bytes_read = stream.read_exact(&mut response);
        
        if bytes_read.is_ok() && &response == b"OK" {
            debug!("Narration request accepted by daemon");
        } else {
            warn!("Daemon did not return OK for narration request");
        }

        Ok(())
    }
}
