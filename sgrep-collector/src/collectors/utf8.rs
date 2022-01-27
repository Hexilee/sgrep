use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use utf8_chars::BufReadCharsExt;

use crate::{Collector, Line};

pub struct UTF8Collector {}

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

    fn collect(&self, path: &Path) -> anyhow::Result<Vec<Line>> {
        BufReader::new(File::open(path)?)
            .lines()
            .enumerate()
            .map(|(i, line)| {
                Ok(Line {
                    position: (i + 1).to_string(),
                    line: line?,
                })
            })
            .collect()
    }
}

impl Default for UTF8Collector {
    fn default() -> Self {
        UTF8Collector {}
    }
}
