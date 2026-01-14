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
    #[serde(skip_serializing_if = "Option::is_none")]
    sound_file: Option<String>,
}

pub struct Narrator {
    socket_path: String,
    hooks_path: std::path::PathBuf,
}

impl Narrator {
    pub fn new() -> Self {
        let hooks_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/Users/fredrikbranstrom"))
            .join(".claude/hooks");
            
        Self {
            socket_path: SOCKET_PATH.to_string(),
            hooks_path,
        }
    }

    /// Send a narration request to the background daemon.
    /// This is non-blocking on the audio playback itself, 
    /// but blocking on the socket communication (which is instant).
    pub fn narrate(&self, text: &str, sound_hint: Option<&str>) -> Result<()> {
        if !Path::new(&self.socket_path).exists() {
            debug!("TTS daemon socket not found at {}, skipping narration", self.socket_path);
            return Ok(());
        }

        // Resolve absolute path if hint is provided
        let resolved_sound = sound_hint.map(|hint| {
            if hint.starts_with("sounds/") {
                self.hooks_path.join(hint).to_string_lossy().to_string()
            } else {
                hint.to_string()
            }
        });

        let request = NarrationRequest {
            request_type: "arbitrary_narration".to_string(),
            text: text.to_string(),
            voice: "lucy".to_string(), // Matches user's preferred voice in config
            sound_file: resolved_sound,
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
