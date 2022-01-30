#![feature(string_remove_matches)]

mod pdf;
mod utf8;

use std::path::Path;

pub use self::pdf::PDFCollector;
pub use self::utf8::UTF8Collector;

#[derive(Debug, Clone, Default)]
pub struct Line {
    pub position: String,
    pub line: String,
}

pub trait Collector: Sync + Send {
    fn name(&self) -> &'static str;
    fn collect(&self, path: &Path) -> anyhow::Result<Vec<Line>>;

    fn accept_extension(&self, _extension: Option<&str>) -> bool {
        // accept all extensions by default
        true
    }

    fn should_collect(&self, path: &Path) -> anyhow::Result<bool> {
        let extension = path.extension().and_then(|e| e.to_str());
        Ok(self.accept_extension(extension.clone()))
    }
}
