use std::path::PathBuf;
use std::str::pattern::Pattern;

use clap::Args;
use colored::Colorize;
use regex::Regex;

use crate::engine::Docs;
use crate::{Command, Engine};

/// Precisely match words by regex
#[derive(Debug, PartialEq, Args)]
pub struct Grep {
    /// Indexing before grep
    #[clap(short, long)]
    indexing: bool,

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
        let pattern = Regex::new(&self.pattern)?;
        let engine = Engine::init(index_dir)?;
        let docs = engine.docs(&self.paths)?;
        for d in docs {
            let doc = d?;
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
                    lines.push((p, line));
                }
            }
            if !lines.is_empty() {
                println!(
                    "{}({})",
                    doc.path().unwrap().purple(),
                    doc.collector().unwrap().yellow().italic()
                );
                for (p, l) in lines {
                    println!("{}:{}", p.green(), l);
                }
                println!("");
            }
        }
        Ok(())
    }
}
