use std::io;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::CustomTransferAgent;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EventJson {
    event: String,
    operation: String,
    remote: String,
    concurrent: bool
}

pub struct Event {
    event: EventType,
    // operation: EventOperation,
    // remote: String,
    // concurrent: bool
}

pub enum EventType {
    Init
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

    fn parse(&self, event: &EventJson) -> Result<Event> {
        let event_type = match event.event.as_str() {
            "init" => EventType::Init,
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
        let event = self.parse(&serde_json::from_str::<EventJson>(buffer.as_str())?)?;

        match event.event {
            EventType::Init => self.custom_transfer_agent.init(&event).await?
        }

        // todo parse from stdin in a loop.

        Ok(())
    }
}