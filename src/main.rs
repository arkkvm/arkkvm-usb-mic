// use usb; // Removed because there is no crate or module named 'usb'

use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::{Duration, timeout};
use tracing::{Level, debug, error, info, warn};
// use crate::mic::AudioOutput;
use arkkvm_mic::usb::mic::AudioOutput;
use arkkvm_mic::usb::mic_sub::AudioSubscriber;
use arkkvm_mic::zenoh_bus;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(true)
        .without_time()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        // .compact()
        .pretty()
        .init();

     tokio::spawn(   async move {
        print!("Initializing Zenoh Bus...\n");
        zenoh_bus::init().await.unwrap();
        AudioSubscriber::new("arkkvm_mic/data", 1024)
        .start()
        .await
        .unwrap();
    });

    println!("app is running. Press Ctrl-C to exit.");
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl-C handler");
   
}
// zenoh

async fn test() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(true)
        .without_time()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        // .compact()
        .pretty()
        .init();

    let args: Vec<String> = env::args().collect();
    let program_name = &args[0];
    info!("Starting program: {} len:{}", program_name, args.len());

    if args.len() == 1 {
        tokio::spawn(async move {
            test_mic_data_task().await;
        });
        let daemon = Daemon::new("./rekvm_mic").arg("start");
        daemon.run();
    } else if args.len() == 2 && args[1] == "start" {
        info!("Audio Output initialized successfully");
        // device.test_task().await;'
        AudioSubscriber::new("rekvm_mic/data", 1024)
            .start()
            .await
            .unwrap();
    }
    // error!("Please provide the path to the audio file as an argument.");
    std::process::exit(1);

    Ok(())
}

struct Daemon {
    program_path: PathBuf,
    args: Vec<String>,
    max_restarts: usize,
    restart_delay: Duration,
}

impl Daemon {

    fn new(program_path: &str) -> Self {
        Self {
            program_path: PathBuf::from(program_path),
            args: Vec::new(),
            max_restarts: 99999,
            restart_delay: Duration::from_secs(1),
        }
    }


    fn with_args(program_path: &str, args: &[&str]) -> Self {
        Self {
            program_path: PathBuf::from(program_path),
            args: args.iter().map(|s| s.to_string()).collect(),
            max_restarts: 10,
            restart_delay: Duration::from_secs(5),
        }
    }


    fn arg<S: AsRef<str>>(mut self, arg: S) -> Self {
        self.args.push(arg.as_ref().to_string());
        self
    }


    fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for arg in args {
            self.args.push(arg.as_ref().to_string());
        }
        self
    }


    fn with_max_restarts(mut self, max_restarts: usize) -> Self {
        self.max_restarts = max_restarts;
        self
    }

    fn with_restart_delay(mut self, seconds: u64) -> Self {
        self.restart_delay = Duration::from_secs(seconds);
        self
    }

    fn run(&self) {
        let mut restart_count = 0;

        while restart_count < self.max_restarts {
            if self.args.is_empty() {
                println!("program_path: {:?}", self.program_path);
            } else {
                println!("program_path: {:?} args: {:?}", self.program_path, self.args);
            }

            // Build the command
            let mut command = Command::new(&self.program_path);
            if !self.args.is_empty() {
                command.args(&self.args);
            }

            match command.spawn() {
                Ok(mut child) => match child.wait() {
                    Ok(status) => {
                        if status.success() {
                            println!("program exited successfully.");
                            break;
                        } else {
                            restart_count += 1;
                            eprintln!(
                                "program exited with code {}. Restarting {}/{}...",
                                status.code().unwrap_or(-1),
                                restart_count,
                                self.max_restarts
                            );

                            thread::sleep(self.restart_delay);
                        }
                    }
                    Err(e) => {
                        restart_count += 1;

                        eprintln!("wait process error: {}. Restarting...", e);
                        thread::sleep(self.restart_delay);
                    }
                },
                Err(e) => {
                    restart_count += 1;

                    eprintln!("lunch error: {}. Restarting...", e);
                    thread::sleep(self.restart_delay);
                }
            }
        }

        if restart_count >= self.max_restarts {
            eprintln!("restart_count is max,stop daemon");
        }
    }
}

async fn test_mic_data_task() {

    let mut config = zenoh::Config::default();
 
    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    let frame_interval = Duration::from_micros(5333); // 5.333ms
    loop {
        use std::fs::File;
        use std::io::Read;
        let file_path = "music.raw";

        info!("Opening audio file: {}", file_path);

        let mut file = File::open(file_path).unwrap();
        info!("Opened audio file: {}", file_path);
        let mut buffer = [0u8; 1024];
        let mut time_stamp = 0;

        println!("Start playing music.raw");
        while file.read(&mut buffer).unwrap() != 0 {
            //let bytes_read = file.read(&mut buffer).unwrap();

            let value = zenoh::bytes::ZBytes::from(buffer.to_vec());
            session.put("rekvm_mic/data", value).await.unwrap();
            thread::sleep(frame_interval);
        }
    }
}
