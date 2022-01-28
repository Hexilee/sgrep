use std::path::PathBuf;

use clap::Args;

use crate::index::Index;
use crate::{Command, Engine};

/// Precisely match words by regex
#[derive(Debug, PartialEq, Args)]
pub struct Grep {
    /// Indexing before grep
    #[clap(short, long)]
    indexing: bool,

    pattern: String,

    /// Paths to index and match, support [glob](https://github.com/rust-lang-nursery/glob)
    #[clap(default_value = "*")]
    paths: String,
}

impl Command for Grep {
    fn run(&self, index_dir: PathBuf) -> anyhow::Result<()> {
        unimplemented!()
    }
}
