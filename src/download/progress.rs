use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering::Relaxed},
};

pub struct ProgressCounter {
    pub(crate) goal: Option<usize>,
    pub(crate) counter: Arc<AtomicUsize>,
}
#[derive(Debug)]
pub struct Progress {
    pub percentage: Option<usize>,
    pub downloaded_mb: usize,
}
impl std::fmt::Display for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}mb", self.downloaded_mb)?;
        if let Some(perc) = self.percentage {
            write!(f, "({perc}%)")?;
        }
        Ok(())
    }
}
impl ProgressCounter {
    pub fn get_bytes(&self) -> usize {
        self.counter.load(Relaxed)
    }
    pub fn get_percentage(&self) -> Option<usize> {
        let prog = self.get_bytes() as f32;
        self.goal
            .map(|goal| ((prog / goal as f32) * 100.0) as usize)
    }
    pub fn get_progress(&self) -> Progress {
        Progress {
            percentage: self.get_percentage(),
            downloaded_mb: self.get_bytes() / 1_000_000,
        }
    }
}
