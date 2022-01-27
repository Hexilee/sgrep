use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Line {
    pub position: String,
    pub line: String,
}

pub trait Collector: Sync + Send {
    fn name(&self) -> &'static str;
    fn should_collect(&self, path: &Path) -> anyhow::Result<bool>;
    fn collect(&self, path: &Path) -> anyhow::Result<Vec<Line>>;
}

pub mod collectors;
