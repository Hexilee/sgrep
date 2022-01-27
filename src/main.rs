#![feature(path_try_exists)]
#![feature(box_syntax)]
#![feature(path_file_prefix)]

use std::borrow::Borrow;
use std::env;
use std::fs::{metadata, try_exists, File};
use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use colored::Colorize;
use glob::glob;
use rayon::prelude::*;
use sgrep_collector::collectors::UTF8Collector;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::{QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::{doc, Index, ReloadPolicy, Snippet, SnippetGenerator, Term};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

mod registry;

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

    let mut schema_builder = Schema::builder();
    let path = schema_builder.add_text_field("path", STRING | STORED);
    let collector = schema_builder.add_text_field("collector", STRING | STORED);
    let hash = schema_builder.add_bytes_field("hash", FAST | STORED);
    let position = schema_builder.add_text_field("position", STRING | STORED);
    let line = schema_builder.add_text_field("line", TEXT | STORED);
    let schema = schema_builder.build();

    let dir = MmapDirectory::open(root.join(INDEX_DIR))?;
    let index = Index::open_or_create(dir, schema.clone())?;
    // Here we use a buffer of 100MB that will be split
    // between indexing threads.
    let index_writer = Arc::new(RwLock::new(index.writer(100_000_000)?));
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()?;
    let searcher = reader.searcher();

    glob(pattern)?
        .par_bridge()
        .filter_map(|p| p.ok())
        .filter_map(|p| {
            let meta = metadata(&p).ok()?;
            if meta.is_file() || meta.is_symlink() {
                Some(p)
            } else {
                None
            }
        })
        .filter(|meta| meta.is_file())
        .map_with(
            (registry.clone(), index_writer.clone()),
            |(reg, index), p| -> anyhow::Result<()> {
                let mut ctx = md5::Context::new();
                std::io::copy(&mut File::open(&p)?, &mut ctx)?;
                let digest = ctx.compute();
                let path_term = Term::from_field_text(path, p.to_string_lossy().as_ref());
                let term_query = TermQuery::new(path_term.clone(), IndexRecordOption::Basic);
                let top_docs = searcher.search(&term_query, &TopDocs::with_limit(1))?;
                if let Some((_score, doc_address)) = top_docs.first() {
                    let doc = searcher.doc(*doc_address)?;
                    let hash = doc.get_first(hash).unwrap().bytes_value().unwrap();
                    if hash == digest.as_ref() {
                        return Ok(());
                    } else {
                        index.read().unwrap().delete_term(path_term);
                    }
                }

                if let Some((co, lines)) = reg.collect(&p) {
                    let mut doc = doc!(
                        path => p.to_str().ok_or_else(|| anyhow!("invalid path"))?,
                        collector => co,
                        hash => digest.as_ref(),
                    );

                    for l in lines {
                        doc.add_text(position, l.position);
                        doc.add_text(line, l.line);
                    }
                    index_writer.read().unwrap().add_document(doc);
                }
                Ok(())
            },
        )
        .collect::<anyhow::Result<Vec<_>>>()?;

    index_writer.write().unwrap().commit()?;
    reader.reload()?;

    let query_parser = QueryParser::for_index(&index, vec![line]);
    let q = query_parser.parse_query(query)?;
    let mut snippet_generator = SnippetGenerator::create(&searcher, &*q, line)?;
    snippet_generator.set_max_num_chars(128); // 128 char for each line
    let top_docs = searcher.search(&q, &TopDocs::with_limit(10))?;
    for (_, addr) in top_docs.iter() {
        let doc = searcher.doc(*addr)?;
        let path = doc.get_first(path).unwrap().text().unwrap();
        let collector = doc.get_first(collector).unwrap().text().unwrap();
        let positions = doc.get_all(position);
        let lines = doc.get_all(line);
        // let contents = snippet_generator.snippet_from_doc(&doc);
        println!("{}({})", path.purple(), collector.yellow().italic());
        for (p, l) in positions.zip(lines) {
            let highlighted_line = highlight(snippet_generator.snippet(l.text().unwrap()));
            if !highlighted_line.is_empty() {
                println!("{}:{}", p.text().unwrap().green(), highlighted_line,);
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

fn highlight(snippet: Snippet) -> String {
    let mut result = String::new();
    let mut start_from = 0;

    for fragment_range in snippet.highlighted() {
        result.push_str(&snippet.fragments()[start_from..fragment_range.start]);
        result.push_str(&format!(
            "{}",
            &snippet.fragments()[fragment_range.clone()].red().bold()
        ));
        start_from = fragment_range.end;
    }

    result.push_str(&snippet.fragments()[start_from..]);
    result
}
