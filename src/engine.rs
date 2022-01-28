use std::fs::{metadata, File};
use std::iter::Iterator;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use cang_jie::{CangJieTokenizer, TokenizerOption};
use glob::glob;
use jieba_rs::Jieba;
use rayon::prelude::*;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::{QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::tokenizer::{Language, Stemmer, StopWordFilter, TextAnalyzer};
use tantivy::{doc, Document, Index, ReloadPolicy, SnippetGenerator, Term};

use crate::registry::Registry;

const TOKENIZER: &str = "jieba-with-filters";

pub struct Engine {
    index_dir: PathBuf,
    index: Index,
    fields: Fields,
    schema: Schema,
}

#[derive(Clone)]
struct Fields {
    path: Field,
    collector: Field,
    hash: Field,
    position: Field,
    line: Field,
}

pub struct Doc<'a> {
    fields: &'a Fields,
    doc: Document,
}

pub type Docs<'a> = Box<dyn 'a + Iterator<Item = anyhow::Result<Doc<'a>>>>;

impl Doc<'_> {
    pub fn path(&self) -> Option<&str> {
        self.doc.get_first(self.fields.path)?.text()
    }

    pub fn collector(&self) -> Option<&str> {
        self.doc.get_first(self.fields.collector)?.text()
    }

    pub fn hash(&self) -> Option<&[u8]> {
        self.doc.get_first(self.fields.hash)?.bytes_value()
    }

    pub fn lines(&self) -> impl Iterator<Item = (&'_ str, &'_ str)> {
        let positions = self.doc.get_all(self.fields.position);
        let lines = self.doc.get_all(self.fields.line);
        positions
            .zip(lines)
            .filter_map(|(p, l)| Some((p.text()?, l.text()?)))
    }
}

impl Engine {
    pub fn init(index_dir: PathBuf) -> anyhow::Result<Self> {
        let line_field_indexing = TextFieldIndexing::default()
            .set_tokenizer(TOKENIZER)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let line_options = TextOptions::default()
            .set_indexing_options(line_field_indexing)
            .set_stored();

        let mut schema_builder = Schema::builder();
        let path = schema_builder.add_text_field("path", STRING | STORED);
        let collector = schema_builder.add_text_field("collector", STRING | STORED);
        let hash = schema_builder.add_bytes_field("hash", FAST | STORED);
        let position = schema_builder.add_text_field("position", STRING | STORED);
        let line = schema_builder.add_text_field("line", line_options);
        let schema = schema_builder.build();

        let dir = MmapDirectory::open(&index_dir)?;
        let index = Index::open_or_create(dir, schema.clone())?;

        let tokenizer = TextAnalyzer::from(CangJieTokenizer {
            worker: Arc::new(Jieba::new()),
            option: TokenizerOption::ForSearch { hmm: false },
        })
        .filter(StopWordFilter::default())
        .filter(Stemmer::new(Language::English));
        index.tokenizers().register(TOKENIZER, tokenizer);
        Ok(Self {
            index_dir,
            index,
            schema,
            fields: Fields {
                path,
                collector,
                hash,
                position,
                line,
            },
        })
    }

    pub fn indexing(
        &mut self,
        registry: &Registry,
        pattern: &str,
        heap_size_in_bytes: usize,
    ) -> anyhow::Result<()> {
        let index_writer = Arc::new(RwLock::new(self.index.writer(heap_size_in_bytes)?));
        let reader = self
            .index
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
                    let path_term =
                        Term::from_field_text(self.fields.path, p.to_string_lossy().as_ref());
                    let term_query = TermQuery::new(path_term.clone(), IndexRecordOption::Basic);
                    let top_docs = searcher.search(&term_query, &TopDocs::with_limit(1))?;
                    if let Some((_score, doc_address)) = top_docs.first() {
                        let doc = searcher.doc(*doc_address)?;
                        let hash = doc
                            .get_first(self.fields.hash)
                            .unwrap()
                            .bytes_value()
                            .unwrap();
                        if hash == digest.as_ref() {
                            return Ok(());
                        } else {
                            index.read().unwrap().delete_term(path_term);
                        }
                    }

                    if let Some((co, lines)) = reg.collect(&p) {
                        let mut doc = doc!(
                            self.fields.path => p.to_str().ok_or_else(|| anyhow!("invalid path"))?,
                            self.fields.collector => co,
                            self.fields.hash => digest.as_ref(),
                        );

                        for l in lines {
                            doc.add_text(self.fields.position, l.position);
                            doc.add_text(self.fields.line, l.line);
                        }
                        index_writer.read().unwrap().add_document(doc);
                    }
                    Ok(())
                },
            )
            .collect::<anyhow::Result<Vec<_>>>()?;
        index_writer.write().unwrap().commit()?;
        Ok(())
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<(Docs<'_>, SnippetGenerator)> {
        let query_parser = QueryParser::for_index(&self.index, vec![self.fields.line]);
        let q = query_parser.parse_query(query)?;
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let searcher = Arc::new(reader.searcher());
        let snippet_generator = SnippetGenerator::create(&searcher, &*q, self.fields.line)?;
        let top_docs = searcher.search(&q, &TopDocs::with_limit(limit))?;
        let docs = top_docs
            .into_iter()
            .map(move |(_, addr)| searcher.clone().doc(addr))
            .map(|d| {
                Ok(Doc {
                    fields: &self.fields,
                    doc: d?,
                })
            });
        Ok((Box::new(docs), snippet_generator))
    }
}
