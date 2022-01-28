use clap::Args;
use tantivy::SnippetGenerator;

use crate::engine::Docs;
use crate::{Engine, Searcher};

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

impl Searcher for Grep {
    fn search<'a>(&self, engine: &'a Engine) -> anyhow::Result<(Docs<'a>, SnippetGenerator)> {
        engine.grep(&self.pattern, 5, &self.paths)
    }
}
