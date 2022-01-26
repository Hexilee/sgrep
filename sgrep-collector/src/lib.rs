use std::path::Path;

pub trait Collector: Sync + Send {
    fn name(&self) -> &'static str;
    fn should_collect(&self, path: &Path) -> anyhow::Result<bool>;
    fn collect(&self, path: &Path) -> anyhow::Result<String>;
}

pub mod collectors;
