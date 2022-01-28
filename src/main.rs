#![feature(path_try_exists)]
#![feature(box_syntax)]

use std::borrow::Borrow;
use std::env;
use std::fs::{metadata, try_exists};
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use colored::Colorize;
use sgrep_collector::collectors::UTF8Collector;
use tracing::debug;
use tracing_subscriber::EnvFilter;

use self::engine::Engine;
use self::highlight::highlight;

mod engine;
mod grep;
mod highlight;
mod index;
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
    sub: Subcommands,
}

#[derive(Debug, PartialEq, Subcommand)]
enum Subcommands {
    Grep(grep::Grep),
    Search(search::Search),
    Index(index::Index),
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .map_err(|err| anyhow::anyhow!("fail to init tracing subscriber: {}", err))?;

    let app = App::parse();

    ensure_all()?;

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(anyhow!("usage: sgrep <query> <path>"));
    }

    let query = &args[1];
    let pattern = args.get(2).map(|v| v.as_str()).unwrap_or("*");

    let registry = registry::Registry::builder()
        .register(UTF8Collector::default())
        .build()?;

    let mut engine = Engine::init(index_dir(&root_dir()?))?;
    engine.indexing(&registry, pattern, 100_000_000)?;
    let (docs, snippet_generator) = engine.search(query, 5, pattern)?;
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
