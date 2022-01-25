use std::path::Path;

pub trait Collector {
    fn name(&self) -> &'static str;
    fn should_collect(&self, path: &Path) -> anyhow::Result<bool>;
    fn collect(&self, path: &Path) -> anyhow::Result<String>;
}
