use std::path::PathBuf;

use clap::Args;
use colored::Colorize;

use crate::highlight::highlight;
use crate::index::Index;
use crate::{Command, Engine};

/// Fuzzy search words
#[derive(Debug, PartialEq, Args)]
pub struct Search {
    /// Indexing before search
    #[clap(short, long)]
    indexing: bool,

    query: String,

    /// Paths to index and match, support [glob](https://github.com/rust-lang-nursery/glob)
    #[clap(default_value = "*")]
    paths: String,
}

impl Command for Search {
    fn run(&self, index_dir: PathBuf) -> anyhow::Result<()> {
        let engine = Engine::init(index_dir)?;
        let (docs, snippet_generator) = engine.search(&self.query, 5, &self.paths)?;
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
