use clap::Args;

/// Precisely match words by regex
#[derive(Debug, PartialEq, Args)]
pub struct Grep {
    /// Indexing before grep
    #[clap(short, long)]
    indexing: bool,

    pattern: String,

    // Paths to index and match, support [glob](https://github.com/rust-lang-nursery/glob)
    #[clap(about, default_value = "*")]
    paths: String,
}
