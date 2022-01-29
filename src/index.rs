use std::path::PathBuf;

use clap::Args;
use sgrep_collector::collectors::UTF8Collector;

use crate::registry::Registry;
use crate::{Command, Engine};

/// Manage indexes
#[derive(Debug, PartialEq, Args)]
pub struct Index {
    /// Delete indexes by paths
    #[clap(short, long)]
    delete: bool,

    /// Delete all indexes
    #[clap(short = 'D', long)]
    delete_all: bool,

    /// Paths to index and match, support [glob](https://github.com/rust-lang-nursery/glob)
    #[clap(default_value = "*")]
    paths: String,
}

impl Command for Index {
    fn run(&self, index_dir: PathBuf) -> anyhow::Result<()> {
        let registry = Registry::builder()
            .register(UTF8Collector::default())
            .build()?;
        let mut engine = Engine::init(index_dir, registry, None)?;
        if self.delete_all {
            engine.remove_all_indexes()
        } else if self.delete {
            engine.remove_indexes(&self.paths)
        } else {
            engine.indexing(&self.paths)
        }
    }
}
