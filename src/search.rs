use std::path::PathBuf;

use clap::Args;
use colored::Colorize;
use sgrep_collector::collectors::UTF8Collector;

use crate::highlight::highlight;
use crate::registry::Registry;
use crate::{Command, Engine};

/// Fuzzy search words
#[derive(Debug, PartialEq, Args)]
pub struct Search {
    /// Top N files to be printed out at most
    #[clap(short, long, default_value = "5")]
    limit: usize,

    /// Indexing before search
    #[clap(short = 'I', long)]
    indexing: bool,

    /// The query in key words
    query: String,

    /// Paths to index and match, supports [glob](https://github.com/rust-lang-nursery/glob)
    #[clap(default_value = "*")]
    paths: String,
}

impl Command for Search {
    fn run(&self, index_dir: PathBuf) -> anyhow::Result<()> {
        let registry = Registry::builder()
            .register(UTF8Collector::default())
            .build()?;
        let mut engine = Engine::init(index_dir, registry, None)?;
        if self.indexing {
            engine.indexing(&self.paths)?;
        }

        let (docs, snippet_generator) = engine.search(&self.query, self.limit, &self.paths)?;
        for d in docs {
            let doc = d?;
            let path = doc.path().unwrap();
            let collector = doc.collector().unwrap();
            println!("{}({})", path.purple(), collector.yellow().italic());
            for (p, l) in doc.lines() {
                if let Some(highlighted_line) = highlight(&snippet_generator, l) {
                    println!("{}:{}", p.green(), highlighted_line);
                }
            }
            println!("");
        }
        Ok(())
    }
}
