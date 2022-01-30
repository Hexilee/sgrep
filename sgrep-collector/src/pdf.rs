use std::path::Path;

use lopdf::Document;
use rayon::prelude::*;
use tracing::instrument;

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
        let doc = Document::load(path)?;
        let mut indexed_pages = doc
            .get_pages()
            .into_iter()
            .par_bridge()
            .map(|(p, _)| {
                let mut page = doc.extract_text(&[p])?;
                page.remove_matches("?Identity-H Unimplemented?");
                Ok((p, page))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        indexed_pages.sort_by(|(p1, _), (p2, _)| p1.cmp(p2));
        Ok(indexed_pages
            .into_iter()
            .map(|(p, page)| Line {
                position: format!("p{}", p),
                line: page,
            })
            .collect())
    }
}
