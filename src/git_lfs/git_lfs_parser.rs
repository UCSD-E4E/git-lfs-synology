use std::io;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

use super::CustomTransferAgent;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitLfsProgressReporter {
    total_bytes: usize,
    bytes_so_far: usize,
    bytes_since_last: usize,
    oid: String,
    event: String
}

impl GitLfsProgressReporter {
    pub fn new(total_bytes: usize, oid: String) -> GitLfsProgressReporter {
        GitLfsProgressReporter {
            total_bytes,
            oid,
            bytes_so_far: 0,
            bytes_since_last: 0,
            event: "progress".to_string()
        }
    }

    pub fn update(&mut self, bytes_since_last: usize) -> Result<()> {
        self.bytes_since_last = bytes_since_last;
        self.bytes_so_far += bytes_since_last;

        let progress_json = serde_json::to_string(self)?;

        info!("Reporting progress: \"{}\".", progress_json);
        println!("{}", progress_json);
        Ok(())
    }
}

pub fn error_init(code: u32, message: &str) -> Result<()> {
    let error_json = ErrorJson {
        error: ErrorJsonInner {
            code,
            message: message.to_string()
        }
    };

    let error_json = serde_json::to_string(&error_json)?;

    error!("Reporting error: \"{}\".", error_json);
    println!("{}", error_json);
    Ok(())
}

pub fn complete_upload(oid: &str) -> Result<()> {
    let complete_json = EventJson {
        event: "complete".to_string(),
        oid: Some(oid.to_string()),
        path: None,
        size: None
    };

    let complete_json = serde_json::to_string(&complete_json)?;

    info!("Reporting complete: \"{}\".", complete_json);
    println!("{}", complete_json);
    Ok(())
}

pub fn complete_download(oid: &str, path: &str) -> Result<()> {
    let complete_json = EventJson {
        event: "complete".to_string(),
        oid: Some(oid.to_string()),
        path: Some(path.to_string()),
        size: None
    };

    let complete_json = serde_json::to_string(&complete_json)?;

    info!("Reporting complete: \"{}\".", complete_json);
    println!("{}", complete_json);
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EventJson {
    event: String,
    oid: Option<String>,
    path: Option<String>,
    size: Option<usize>
}

#[derive(Debug)]
pub struct Event {
    pub event: EventType,
    pub oid: Option<String>,
    pub path: Option<String>,
    pub size: Option<usize>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ErrorJson {
    error: ErrorJsonInner
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ErrorJsonInner {
    code: u32,
    message: String
}

#[derive(Debug)]
pub enum EventType {
    Download,
    Init,
    Terminate,
    Upload
}

#[derive(Debug)]
pub struct GitLfsParser<'custom_transfer_agent, T: CustomTransferAgent> {
    custom_transfer_agent: &'custom_transfer_agent mut T
}

impl<'custom_transfer_agent, T: CustomTransferAgent> GitLfsParser<'custom_transfer_agent, T> {
    #[tracing::instrument]
    pub fn new(custom_transfer_agent: &mut T) -> GitLfsParser::<T> {
        GitLfsParser::<T> {
            custom_transfer_agent
        }
    }

    #[tracing::instrument]
    fn parse(&self, event: &EventJson) -> Result<Event> {
        info!("Event received: \"{}\".", event.event);
        let event_type = match event.event.as_str() {
            "download" => EventType::Download,
            "init" => EventType::Init,
            "terminate" => EventType::Terminate,
            "upload" => EventType::Upload,
            _ => bail!("Event type was \"{}\". Value unexpected.", event.event)
        };

        Ok(Event {
            event: event_type,
            oid: event.oid.clone(),
            path: event.path.clone(),
            size: event.size
        })
    }

    #[tracing::instrument]
    pub async fn listen(&mut self) -> Result<()> {
        let mut buffer = String::new();
        let stdin = io::stdin();

        stdin.read_line(&mut buffer)?;
        info!("Received JSON: \"{}\".", buffer);
        let event = self.parse(&serde_json::from_str::<EventJson>(buffer.as_str())?)?;
        buffer.clear();

        let init_result = match event.event {
            EventType::Init => {
                info!("Calling init on custom transfer agent.");
                self.custom_transfer_agent.init(&event).await
            },
            _ => bail!("Event type was not init.")
        };

        match init_result {
            Ok(_) => {
                info!("Init event parsed correctly.");

                println!("{{ }}")
            }, // success
            Err(err) => {
                warn!("An error occurred \"{}\".", err);

                error_init(1, err.to_string().as_str())? // an error occurred
            }
        }

        loop {
            stdin.read_line(&mut buffer)?;
            info!("Received JSON: \"{}\".", buffer);
            let event = self.parse(&serde_json::from_str::<EventJson>(buffer.as_str())?)?;
            buffer.clear();

            match event.event {
                EventType::Download => {
                    info!("Calling download on custom transfer agent.");
                    let path = self.custom_transfer_agent.download(&event).await?;

                    complete_download(
                        event.oid.context("OID should not be null")?.as_str(),
                        path.as_os_str().to_str().context("Path should not be null")?)?;
                },
                EventType::Upload => {
                    info!("Calling upload on custom transfer agent.");
                    self.custom_transfer_agent.upload(&event).await?;

                    complete_upload(event.oid.context("OID should not be null")?.as_str())?;
                },
                EventType::Terminate => {
                    info!("Calling terminate on custom transfer agent.");
                    self.custom_transfer_agent.terminate().await?;

                    break;
                },
                _ => {
                    warn!("Event type not supported in context.");

                    bail!("Event type not supported in context.")
                }
            }
        }

        Ok(())
    }
}