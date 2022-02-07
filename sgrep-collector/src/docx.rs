use std::io::{BufRead, BufReader};
use std::path::Path;

use dotext::{Docx, MsDoc};
use tracing::instrument;

use crate::{Collector, Line};

#[derive(Debug, Clone, Copy)]
pub struct DocxCollector;

impl Collector for DocxCollector {
    fn name(&self) -> &'static str {
        "docx"
    }

    fn accept_extension(&self, extension: Option<&str>) -> bool {
        matches!(extension, Some(e) if e == "docx" || e == "doc")
    }

    #[instrument]
    fn collect(&self, path: &Path) -> anyhow::Result<Vec<Line>> {
        let mut doc = Docx::open(path)?;
        let buffered = BufReader::new(&mut doc);
        buffered
            .lines()
            .enumerate()
            .map(|(_, line)| {
                Ok(Line {
                    position: "".to_string(), // TODO: locate lines
                    line: line?,
                })
            })
            .collect()
    }
}
