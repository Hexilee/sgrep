use clap::Args;
use tantivy::SnippetGenerator;

use crate::engine::Docs;
use crate::{Engine, Searcher};

/// Fuzzy search words
#[derive(Debug, PartialEq, Args)]
pub struct Search {
    /// Indexing before search
    #[clap(short, long)]
    indexing: bool,

    /// The query in key words
    query: String,

    /// Paths to index and match, supports [glob](https://github.com/rust-lang-nursery/glob)
    #[clap(default_value = "*")]
    paths: String,
}

impl Searcher for Search {
    fn search<'a>(&self, engine: &'a Engine) -> anyhow::Result<(Docs<'a>, SnippetGenerator)> {
        engine.search(&self.query, 5, &self.paths)
    }
}
