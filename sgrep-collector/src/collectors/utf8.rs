use std::fs::{read_to_string, File};
use std::io::BufReader;
use std::path::Path;

use utf8_chars::BufReadCharsExt;

use crate::Collector;

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

    fn collect(&self, path: &Path) -> anyhow::Result<String> {
        let contents = read_to_string(path)?;
        Ok(contents)
    }
}

impl Default for UTF8Collector {
    fn default() -> Self {
        UTF8Collector {}
    }
}
