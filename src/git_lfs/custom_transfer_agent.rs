use std::fmt::Debug;

use anyhow::Result;

use super::git_lfs_parser::Event;

pub trait CustomTransferAgent : Debug {
    async fn download(&mut self, event: &Event) -> Result<()>;
    async fn init(&mut self, event: &Event) -> Result<()>;
    async fn terminate(&mut self) -> Result<()>;
    async fn upload(&mut self, event: &Event) -> Result<()>;
}