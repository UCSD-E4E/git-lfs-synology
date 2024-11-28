use std::io;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::CustomTransferAgent;

pub fn error_init(code: u32, message: &str) -> Result<()> {
    let error_json = ErrorJson {
        error: ErrorJsonInner {
            code,
            message: message.to_string()
        }
    };

    println!("{}", serde_json::to_string(&error_json)?);
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EventJson {
    event: String
}

pub struct Event {
    event: EventType
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

pub enum EventType {
    // Complete,
    Download,
    Init,
    // Progress,
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
            event: event_type
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
                    self.custom_transfer_agent.download(&event).await?
                },
                EventType::Upload => {
                    info!("Calling upload on custom transfer agent.");
                    self.custom_transfer_agent.upload(&event).await?
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