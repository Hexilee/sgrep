use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use tracing::{debug, instrument};
use utf8_chars::BufReadCharsExt;

use crate::{Collector, Line};

#[derive(Debug, Clone, Copy)]
pub struct UTF8Collector;

impl Collector for UTF8Collector {
    fn name(&self) -> &'static str {
        "utf8"
    }

    fn should_collect(&self, path: &Path) -> anyhow::Result<bool> {
        let mut f = BufReader::new(File::open(path)?);
        for c in f.chars() {
            if c.is_err() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[instrument]
    fn collect(&self, path: &Path) -> anyhow::Result<Vec<Line>> {
        BufReader::new(File::open(path)?)
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let l = Line {
                    position: (i + 1).to_string(),
                    line: line?,
                };
                debug!("collect line: {:?}", l);
                Ok(l)
            })
            .collect()
    }
}
