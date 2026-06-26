// mic.rs
use super::mic_c::{AudioOutputConfig, AudioOutputContext, AudioOutputError, AudioProcessorC};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uchar, c_uint};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::mpsc;
use tokio::task;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};
use zenoh::config::Config;

pub struct AudioOutput {
    apc: AudioProcessorC,
}

impl AudioOutput {
    pub fn new(
        card_name: &str,
        sample_rate: i32,
        channels: i32,
        format: &str,
    ) -> Result<Self, AudioOutputError> {
        let mut apc = AudioProcessorC::new();

        let config = AudioOutputConfig {
            card_name: Some(CString::new(card_name).unwrap()),
            sample_rate: sample_rate as u32,
            channels: channels as u32,
            format: Some(CString::new(format).unwrap()),
        };
        apc.set_config(config);
        let info = apc.init();

        match info {
            AudioOutputError::Success => { 
            }
            _ => {
                eprintln!("Failed to initialize audio output: {:?}", info);
                std::thread::sleep(Duration::from_secs(2));
                std::process::exit(1);
            }
        }

        Ok(Self { apc })
    }

    pub fn send_data(&self, data: &[u8]) -> Result<(), AudioOutputError> {
        // info!("Sending audio data of size: {}", data.len());

        let result = self.apc.send_data(data);

        match result {
            AudioOutputError::Success => Ok(()),
            err => Err(err),
        }
    }

    /**
     * file_path: current only support raw pcm file
     */
    pub async fn test_task(&self) -> Result<(), AudioOutputError> {
        use std::fs::File;
        use std::io::Read;
        let file_path = "music.raw";

        info!("Opening audio file: {}", file_path);

        let mut file = match File::open(file_path) {
            Ok(f) => f,
            Err(e) => {
                info!("Failed to open audio file {}: {}", file_path, e);
                return Err(AudioOutputError::ErrorFile);
            }
        };

        info!("Opened audio file: {}", file_path);
        let mut buffer = [0u8; 1024];
        let mut time_stamp = 0;

        println!("Start playing music.raw");
        loop {
            let bytes_read = file.read(&mut buffer).unwrap();
            if bytes_read == 0 {
                break; // End of file
            }
            self.send_data(&buffer[..bytes_read]).unwrap();
        }

        Ok(())
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        unsafe {
            self.apc.release();
        }
    }
}
