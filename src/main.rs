#![feature(path_try_exists)]
#![feature(box_syntax)]

use std::borrow::Borrow;
use std::fs::{metadata, try_exists};
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use tracing::debug;
use tracing_subscriber::filter::LevelFilter;

use self::engine::Engine;

mod engine;
mod grep;
mod highlight;
pub mod index;
pub mod registry;
mod search;

const META_DIR: &str = "sgrep";
const INDEX_DIR: &str = "sgrep/index";

/// Super Grep, search words in everything
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct App {
    /// Verbose level of logs
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Debug, PartialEq, Subcommand)]
enum Commands {
    Grep(grep::Grep),
    Search(search::Search),
    Index(index::Index),
}

trait Command {
    fn run(&self, index_dir: PathBuf) -> anyhow::Result<()>;
}

impl App {
    fn get_level_filter(&self) -> LevelFilter {
        match self.verbose {
            0 => LevelFilter::ERROR,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        }
    }

    fn get_command(&self) -> &dyn Command {
        use Commands::*;
        match &self.commands {
            Grep(c) => &*c,
            Search(c) => &*c,
            Index(c) => &*c,
        }
    }
}

impl Command for App {
    fn run(&self, index_dir: PathBuf) -> anyhow::Result<()> {
        ensure_all()?;
        self.get_command().run(index_dir)
    }
}

fn main() -> anyhow::Result<()> {
    let app: App = Parser::parse();
    tracing_subscriber::fmt()
        .with_max_level(app.get_level_filter())
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|err| anyhow!("{}", err))?;

    app.run(index_dir(&root_dir()?))
}

fn ensure_dir(path: impl AsRef<Path>) -> anyhow::Result<()> {
    debug!("ensure dir: {:?}", path.as_ref());
    if !try_exists(path.as_ref())? {
        std::fs::create_dir(path.borrow())?;
    }

    if !metadata(path.borrow())?.is_dir() {
        Err(anyhow!("{:?} is not a directory", path.as_ref()))
    } else {
        Ok(())
    }
}

fn ensure_all() -> anyhow::Result<()> {
    let root = root_dir()?;
    ensure_dir(&meta_dir(&root))?;
    ensure_dir(&index_dir(&root))
}

fn root_dir() -> anyhow::Result<PathBuf> {
    dirs::data_dir().ok_or_else(|| anyhow::anyhow!("fail to get data dir"))
}

fn index_dir(root: &Path) -> PathBuf {
    root.join(INDEX_DIR)
}

fn meta_dir(root: &Path) -> PathBuf {
    root.join(META_DIR)
}
