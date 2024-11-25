use std::io;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::CustomTransferAgent;

pub fn error(code: u32, message: &str) -> Result<()> {
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
struct EventJsonPartial {
    event: String
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

pub struct Event {
    event: EventType,
    // operation: EventOperation,
    // remote: String,
    // concurrent: bool
}

pub enum EventType {
    // Complete,
    Download,
    Init,
    // Progress,
    Terminate,
    Upload
}

// pub enum EventOperation {
//     Download,
//     Upload
// }

#[derive(Debug)]
pub struct GitLfsParser<'custom_transfer_agent, T: CustomTransferAgent> {
    custom_transfer_agent: &'custom_transfer_agent mut T
}

impl<'custom_transfer_agent, T: CustomTransferAgent> GitLfsParser<'custom_transfer_agent, T> {
    pub fn new(custom_transfer_agent: &mut T) -> GitLfsParser::<T> {
        GitLfsParser::<T> {
            custom_transfer_agent
        }
    }

    fn parse(&self, event: &EventJsonPartial) -> Result<Event> {
        let event_type = match event.event.as_str() {
            "download" => EventType::Download,
            "init" => EventType::Init,
            "terminate" => EventType::Terminate,
            "upload" => EventType::Upload,
            _ => bail!("Event type was \"{}\". Value unexpected.", event.event)
        };

        // let event_operation = match event.operation.as_str() {
        //     "download" => EventOperation::Download,
        //     "upload" => EventOperation::Upload,
        //     _ => bail!("Event operation was \"{}\". Expected either \"download\" or \"upload\".", event.operation)
        // };

        Ok(Event {
            event: event_type,
            // operation: event_operation,
            // remote: event.remote.clone(),
            // concurrent: event.concurrent
        })
    }

    pub async fn listen(&mut self) -> Result<()> {
        let mut buffer = String::new();
        let stdin = io::stdin();

        stdin.read_line(&mut buffer)?;
        let event = self.parse(&serde_json::from_str::<EventJsonPartial>(buffer.as_str())?)?;

        let init_result = match event.event {
            EventType::Init => self.custom_transfer_agent.init(&event).await,
            _ => bail!("Event type was not init.")
        };

        match init_result {
            Ok(_) => println!("{{ }}"), // success
            Err(err) => error(1, err.to_string().as_str())? // an error occurred
        }

        loop {
            stdin.read_line(&mut buffer)?;
            let event = self.parse(&serde_json::from_str::<EventJsonPartial>(buffer.as_str())?)?;

            match event.event {
                EventType::Download => self.custom_transfer_agent.download(&event).await?,
                EventType::Upload => self.custom_transfer_agent.upload(&event).await?,
                EventType::Terminate => {
                    self.custom_transfer_agent.terminate().await?;

                    break;
                },
                _ => bail!("Event type not supported in context.")
            }
        }

        Ok(())
    }
}