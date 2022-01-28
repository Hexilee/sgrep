use clap::Args;

/// Manage indexes
#[derive(Debug, PartialEq, Args)]
pub struct Index {
    /// Delete indexes by pattern
    #[clap(short, long)]
    delete: bool,

    /// Delete all indexes
    #[clap(short = 'D', long)]
    delete_all: bool,
}
