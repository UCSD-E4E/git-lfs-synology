use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::CustomTransferAgent;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde[rename_all = "snake_case"]]
struct Event {
    event: String,
    operation: String,
    remote: String,
    concurrent: bool
}

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

    pub async fn listen(&mut self) -> Result<()> {
        // todo parse from stdin in a loop.

        self.custom_transfer_agent.init().await?;

        Ok(())
    }
}