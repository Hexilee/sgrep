use std::path::Path;

use pdf::file::File;
use tracing::{debug, instrument};

use crate::{Collector, Line};

#[derive(Debug, Clone, Copy)]
pub struct PDFCollector;

impl Collector for PDFCollector {
    fn name(&self) -> &'static str {
        "pdf"
    }

    fn accept_extension(&self, extension: Option<&str>) -> bool {
        match extension {
            Some(e) if e == "pdf" => true,
            _ => false,
        }
    }

    #[instrument]
    fn collect(&self, path: &Path) -> anyhow::Result<Vec<Line>> {
        let file = File::open(path)?;
        let mut lines = vec![];
        for (i, p) in file.pages().enumerate() {
            let page = p?;
            let contents = match page.contents {
                None => continue,
                Some(ref c) => c,
            };
            let line = contents
                .operations
                .iter()
                .flat_map(|op| op.operands.iter().map(|p| p.to_string()))
                .collect::<Vec<_>>()
                .join(" ");
            let l = Line {
                position: format!("p{}", i + 1),
                line,
            };
            debug!("collect line: {:?}", l);
            lines.push(l)
        }
        Ok(lines)
    }
}
