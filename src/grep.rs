use std::path::PathBuf;

use clap::Args;
use colored::Colorize;
use rayon::prelude::*;
use regex::Regex;
use sgrep_collector::{PDFCollector, UTF8Collector};

use crate::registry::Registry;
use crate::{Command, Engine};

/// Precisely match words by regex
#[derive(Debug, PartialEq, Args)]
pub struct Grep {
    /// Indexing before grep
    #[clap(short = 'I', long)]
    indexing: bool,

    /// Perform case insensitive matching.  By default, sgrep is case sensitive.
    #[clap(short, long)]
    ignore_case: bool,

    /// The pattern, support regex
    pattern: String,

    /// Paths to index and match, support [glob](https://github.com/rust-lang-nursery/glob)
    #[clap(default_value = "*")]
    paths: String,
}

// // Not work now
// impl Searcher for Grep {
//     fn search<'a>(&self, engine: &'a Engine) -> anyhow::Result<(Docs<'a>, SnippetGenerator)> {
//         engine.grep(&self.pattern, 5, &self.paths)
//     }
// }

impl Command for Grep {
    fn run(&self, index_dir: PathBuf) -> anyhow::Result<()> {
        let pattern = if !self.ignore_case {
            Regex::new(&self.pattern)?
        } else {
            Regex::new(&format!(r"(?i){}", self.pattern))?
        };
        let registry = Registry::builder()
            .register(PDFCollector)
            .register(UTF8Collector)
            .build()?;
        let mut engine = Engine::init(index_dir, registry, None)?;
        if self.indexing {
            engine.indexing(&self.paths)?;
        }
        let docs = engine
            .docs(&self.paths)?
            .par_bridge()
            .filter_map(|d| {
                let doc = d.ok()?;
                let mut lines = Vec::new();
                for (p, l) in doc.lines() {
                    let indices = l.match_indices(&pattern).collect::<Vec<_>>();
                    if !indices.is_empty() {
                        let mut line = l.to_string();
                        for (i, fragment) in indices {
                            line.replace_range(
                                i..i + fragment.len(),
                                &format!("{}", fragment.red().bold()),
                            )
                        }
                        lines.push((p.to_string(), line));
                    }
                }
                if lines.is_empty() {
                    None
                } else {
                    Some((
                        doc.path().unwrap().to_string(),
                        doc.collector().unwrap().to_string(),
                        lines,
                    ))
                }
            })
            .collect::<Vec<_>>();
        for (path, collector, lines) in docs {
            println!("{}({})", path.purple(), collector.yellow().italic());
            for (p, l) in lines {
                println!("{}:{}", p.green(), l);
            }
            println!("");
        }
        Ok(())
    }
}
