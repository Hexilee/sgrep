use clap::Args;

/// Fuzzy search words
#[derive(Debug, PartialEq, Args)]
pub struct Search {
    /// Indexing before search
    #[clap(short, long)]
    indexing: bool,

    pattern: String,

    // Paths to index and match, support [glob](https://github.com/rust-lang-nursery/glob)
    #[clap(about, default_value = "*")]
    paths: String,
}
