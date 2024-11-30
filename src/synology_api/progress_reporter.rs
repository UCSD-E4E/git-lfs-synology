use std::fmt::Debug;

use anyhow::Result;

pub trait ProgressReporter: Send + Debug {
    fn update(&mut self, bytes_since_last: usize) -> Result<()>;
}