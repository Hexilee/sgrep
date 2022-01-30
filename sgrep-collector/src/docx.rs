use std::path::Path;

use docx::document::{BodyContent, Paragraph};
use docx::DocxFile;
use rayon::prelude::*;
use tracing::instrument;

use crate::{Collector, Line};

#[derive(Debug, Clone, Copy)]
pub struct DocxCollector;

impl Collector for DocxCollector {
    fn name(&self) -> &'static str {
        "docx"
    }

    fn accept_extension(&self, extension: Option<&str>) -> bool {
        match extension {
            Some(e) if e == "docx" || e == "doc" => true,
            _ => false,
        }
    }

    #[instrument]
    fn collect(&self, path: &Path) -> anyhow::Result<Vec<Line>> {
        let doc_file = DocxFile::from_file(path)?;
        let doc = doc_file.parse()?;
        let mut indexed_pages = doc
            .document
            .body
            .content
            .into_iter()
            .enumerate()
            .par_bridge()
            .filter_map(|(i, content)| match content {
                BodyContent::Paragraph(p) => Some((i, p)),
                _ => None, // TODD: support tables
            })
            .map(|(i, p)| {
                let page = p.iter_text().fold(String::new(), |page, part| {
                    page.push_str(&part);
                    page.push_str("\n")
                });
                (i, page)
            })
            .collect::<Vec<_>>();
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
