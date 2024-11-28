use anyhow::Result;

pub trait ProgressReporter {
    fn update(&mut self, bytes_so_far: usize, total_bytes: usize) -> Result<()>;
}