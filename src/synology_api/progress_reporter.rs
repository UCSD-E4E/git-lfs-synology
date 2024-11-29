use anyhow::Result;

pub trait ProgressReporter: Send {
    fn update(&mut self, bytes_so_far: usize) -> Result<()>;
}