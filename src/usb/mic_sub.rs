// mic_sub.rs
use crate::zenoh_bus;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use zenoh::config::Config;

use super::mic::AudioOutput;
use super::mic_c::AudioOutputError;
/// Audio subscriber that subscribes to audio data from Zenoh and plays it
pub struct AudioSubscriber {
    // audio_output: Arc<AudioOutput>,
    topic: String,
    buffer_size: usize,
}

impl AudioSubscriber {
    /// Creates a new audio subscriber
    pub fn new(topic: &str, buffer_size: usize) -> Self {
        Self {
            // audio_output: Arc::new(audio_output),
            topic: topic.to_string(),
            buffer_size,
        }
    }

    /// Starts subscribing to audio data
    pub async fn start(&self) -> Result<(), AudioOutputError> {
        let session = zenoh_bus::get_session();

        // Reduce channel capacity to lower latency
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1); // Smaller buffer size

        // Process directly without spawning an extra task
        let mut subscriber = session
            .declare_subscriber("arkkvm_mic/data")
            .callback(move |sample| {
                // if let Some(t) = sample.timestamp() {
                //     // info!("Received audio sample with timestamp: {}", t);
                //     let now = std::time::SystemTime::now()
                //         .duration_since(std::time::UNIX_EPOCH)
                //         .expect("Time went backwards")
                //         .as_millis() as i64;


                //     let delay_ms = now - t.get_time().to_duration().as_millis() as i64;
                //       info!("Audio sample delay={}ms", delay_ms);
                // }

                let data = sample.payload().to_bytes().to_vec();
                if tx.try_send(data).is_err() {
                    // Non-blocking send; drop data if the channel is full
                    // warn!("Audio channel full, dropping packet");
                }
            })
            .await
            .expect("Failed to declare subscriber");

        // Audio output
        let device = AudioOutput::new("hw:1,0", 48000, 2, "S16")?;

        // Simplified processing loop
        // let mut last_warn = std::time::Instant::now();
        while let Some(data) = rx.recv().await {
            if data.is_empty() {
                continue;
            }

            // Use non-blocking send
            if let Err(e) = device.send_data(&data) {
         
            }
        }

      
        Ok(())
    }

    /// Returns the subscribed topic
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Returns the expected buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

impl Drop for AudioSubscriber {
    fn drop(&mut self) {
        info!("AudioSubscriber dropped");
    }
}
