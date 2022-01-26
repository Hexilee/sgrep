#![feature(path_try_exists)]
#![feature(box_syntax)]
#![feature(path_file_prefix)]

use std::collections::HashMap;
use std::env;
use std::fs::{metadata, try_exists, File};
use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use glob::glob;
use rayon::prelude::*;
use sgrep_collector::collectors::UTF8Collector;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, ReloadPolicy};

mod registry;

const META_DIR: &str = "~/.sgrep";
const INDEX_DIR: &str = "~/.sgrep/index";

fn main() -> anyhow::Result<()> {
    ensure_dir(META_DIR)?;
    ensure_dir(INDEX_DIR)?;

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        return Err(anyhow!("usage: sgrep <query> <path>"));
    }

    let query = &args[1];
    let pattern = &args[2];

    let registry = registry::Registry::builder()
        .register(UTF8Collector::default())
        .build()?;

    let mut schema_builder = Schema::builder();
    let path = schema_builder.add_text_field("path", STRING | STORED);
    let collector = schema_builder.add_text_field("collector", STRING | STORED);
    let hash = schema_builder.add_bytes_field("hash", FAST | STORED);
    let filename = schema_builder.add_text_field("filename", TEXT | STORED);
    let contents = schema_builder.add_text_field("contents", TEXT);
    let schema = schema_builder.build();

    let dir = MmapDirectory::open(INDEX_DIR)?;
    let index = Index::open_or_create(dir, schema.clone())?;
    // Here we use a buffer of 100MB that will be split
    // between indexing threads.
    let index_writer = Arc::new(RwLock::new(index.writer(100_000_000)?));

    glob(pattern)?
        .par_bridge()
        .filter_map(|p| p.ok())
        .map_with(
            (registry.clone(), index_writer.clone()),
            |(reg, index), p| -> anyhow::Result<()> {
                let mut ctx = md5::Context::new();
                std::io::copy(&mut File::open(&p)?, &mut ctx)?;
                let digest = ctx.compute();
                let name = p.file_prefix().and_then(|p| p.to_str()).unwrap_or("");
                for (co, collected) in reg.collect(&p)? {
                    // Let's index one documents!
                    index_writer.read().unwrap().add_document(doc!(
                        path => p.to_str().ok_or_else(|| anyhow!("invalid path"))?,
                        collector => co,
                        hash => digest.as_ref(),
                        filename => name,
                        contents => collected,
                    ));
                }
                Ok(())
            },
        )
        .collect::<anyhow::Result<Vec<_>>>()?;

    index_writer.write().unwrap().commit()?;

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()?;
    let searcher = reader.searcher();
    let query_parser = QueryParser::for_index(&index, vec![filename, contents]);
    let q = query_parser.parse_query(query)?;
    let top_docs = searcher.search(&q, &TopDocs::with_limit(10))?;
    for (_, addr) in top_docs.iter() {
        let doc = searcher.doc(*addr)?;
        let path = doc.get_first(path).unwrap().text().unwrap();
        let collector = doc.get_first(collector).unwrap().text().unwrap();
        let hash = doc.get_first(hash).unwrap().bytes_value().unwrap();
        let filename = doc.get_first(filename).unwrap().text().unwrap();
        println!("{}: ", path);
        println!("  {}", collector);
        println!("  {}", filename);
        println!("  {:?}", hash);
    }
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
