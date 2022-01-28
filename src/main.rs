#![feature(path_try_exists)]
#![feature(box_syntax)]

use std::borrow::Borrow;
use std::env;
use std::fs::{metadata, try_exists};
use std::path::Path;

use anyhow::anyhow;
use colored::Colorize;
use sgrep_collector::collectors::UTF8Collector;
use tantivy::SnippetGenerator;
use tracing::debug;
use tracing_subscriber::EnvFilter;

use self::engine::Engine;

pub mod engine;
pub mod registry;

const META_DIR: &str = "sgrep";
const INDEX_DIR: &str = "sgrep/index";

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .map_err(|err| anyhow::anyhow!("fail to init tracing subscriber: {}", err))?;

    let root = dirs::data_dir().ok_or_else(|| anyhow::anyhow!("fail to get data dir"))?;

    ensure_dir(root.join(META_DIR))?;
    ensure_dir(root.join(INDEX_DIR))?;

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(anyhow!("usage: sgrep <query> <path>"));
    }

    let query = &args[1];
    let pattern = args.get(2).map(|v| v.as_str()).unwrap_or("*");

    let registry = registry::Registry::builder()
        .register(UTF8Collector::default())
        .build()?;

    let mut engine = Engine::init(root.join(INDEX_DIR))?;
    engine.indexing(&registry, pattern, 100_000_000)?;
    let (docs, snippet_generator) = engine.search(query, 5)?;
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

fn ensure_dir(path: impl Borrow<Path>) -> anyhow::Result<()> {
    debug!("ensure dir: {:?}", path.borrow());
    if !try_exists(path.borrow())? {
        std::fs::create_dir(path.borrow())?;
    }

    if !metadata(path.borrow())?.is_dir() {
        Err(anyhow!("{:?} is not a directory", path.borrow()))
    } else {
        Ok(())
    }
}

fn highlight(generator: &SnippetGenerator, text: &str) -> Option<String> {
    let snippet = generator.snippet(text);
    if snippet.fragments().is_empty() {
        return None;
    }

    let offset = match text.find(snippet.fragments()) {
        Some(i) => i,
        None => return None,
    };

    let mut result = String::with_capacity(text.len());
    result.push_str(&text[0..offset]);
    let mut start_from = 0;

    for fragment_range in snippet.highlighted() {
        result.push_str(&snippet.fragments()[start_from..fragment_range.start]);
        result.push_str(&format!(
            "{}",
            &snippet.fragments()[fragment_range.clone()].red().bold()
        ));
        start_from = fragment_range.end;
    }

    result.push_str(&text[start_from + offset..]);
    Some(result)
}
