use anyhow::Result;

use super::git_lfs_parser::Event;

pub trait CustomTransferAgent {
    async fn init(&mut self, event: &Event) -> Result<()>;
}