use anyhow::Result;

pub trait CustomTransferAgent {
    async fn init(&mut self) -> Result<()>;
}