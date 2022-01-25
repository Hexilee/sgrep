#![feature(path_try_exists)]

use std::collections::HashMap;
use std::fs::{metadata, try_exists};
use std::path::Path;

use anyhow::anyhow;
use sgrep_collector::Collector;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, ReloadPolicy};

const META_DIR: &str = "./.rgrep";
const INDEX_DIR: &str = "./.rgrep/index";

fn main() -> anyhow::Result<()> {
    ensure_dir(META_DIR)?;
    ensure_dir(INDEX_DIR)?;

    let mut collectors: HashMap<&str, &dyn Collector> = HashMap::new();

    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("path", TEXT | STORED);
    schema_builder.add_text_field("content", TEXT);
    let schema = schema_builder.build();

    let dir = MmapDirectory::open(INDEX_DIR)?;
    let index = Index::open_or_create(dir, schema.clone())?;
    // Here we use a buffer of 100MB that will be split
    // between indexing threads.
    let mut index_writer = index.writer(100_000_000)?;
    Ok(())
}

fn ensure_dir(path: &str) -> anyhow::Result<()> {
    if !try_exists(path)? {
        std::fs::create_dir(path)?;
    }

    if !metadata(path)?.is_dir() {
        Err(anyhow!("{} is not a directory", path))
    } else {
        Ok(())
    }
}
