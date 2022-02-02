use std::collections::HashSet;
use std::fs::{metadata, File};
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use cang_jie::{CangJieTokenizer, TokenizerOption};
use glob::glob;
use jieba_rs::Jieba;
use rayon::prelude::*;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::{Query, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::tokenizer::{Language, LowerCaser, Stemmer, TextAnalyzer};
use tantivy::{doc, Document, Index, ReloadPolicy, SegmentReader, SnippetGenerator, Term};

use self::stopwords::StopWordFilter;
use crate::registry::Registry;

mod stopwords;

const TOKENIZER: &str = "jieba-with-filters";
const DEFAULT_HEAP_SIZE: usize = 100_000_000;

pub struct Engine {
    heap_size: usize,
    registry: Registry,
    index: Index,
    fields: Fields,
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

pub type Docs<'a> = Box<dyn 'a + Send + Iterator<Item = anyhow::Result<Doc<'a>>>>;

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
    pub fn init(
        index_dir: PathBuf,
        registry: Registry,
        heap_size: Option<usize>,
    ) -> anyhow::Result<Self> {
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
        .filter(LowerCaser)
        .filter(StopWordFilter::default())
        .filter(Stemmer::new(Language::English));
        index.tokenizers().register(TOKENIZER, tokenizer);
        Ok(Self {
            heap_size: heap_size.unwrap_or(DEFAULT_HEAP_SIZE),
            registry,
            index,
            fields: Fields {
                path,
                collector,
                hash,
                position,
                line,
            },
        })
    }

    pub fn indexing(&mut self, paths: HashSet<&str>) -> anyhow::Result<()> {
        let index_writer = Arc::new(RwLock::new(self.index.writer(self.heap_size)?));
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let searcher = reader.searcher();
        Self::glob(paths)
            .map_with(
                (
                    self.registry.clone(),
                    index_writer.clone(),
                    self.fields.clone(),
                ),
                |(reg, index, fields), p| -> anyhow::Result<()> {
                    let mut ctx = md5::Context::new();
                    std::io::copy(&mut File::open(&p)?, &mut ctx)?;
                    let digest = ctx.compute();
                    let path_term =
                        Term::from_field_text(self.fields.path, p.to_string_lossy().as_ref());
                    let term_query = TermQuery::new(path_term.clone(), IndexRecordOption::Basic);
                    let top_docs = searcher.search(&term_query, &TopDocs::with_limit(1))?;
                    if let Some((_score, doc_address)) = top_docs.first() {
                        let doc = Doc {
                            fields: &fields,
                            doc: searcher.doc(*doc_address)?,
                        };
                        let hash = doc.hash().unwrap();
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

    pub fn remove_indexes(&mut self, paths: HashSet<&str>) -> anyhow::Result<()> {
        let index_writer = Arc::new(RwLock::new(self.index.writer(self.heap_size)?));
        self.docs(paths)?
            .par_bridge()
            .filter_map(|p| p.ok())
            .for_each(|doc| {
                let path_term = Term::from_field_text(self.fields.path, doc.path().unwrap());
                index_writer.clone().read().unwrap().delete_term(path_term);
            });
        index_writer.write().unwrap().commit()?;
        Ok(())
    }

    pub fn remove_all_indexes(&mut self) -> anyhow::Result<()> {
        let mut index_writer = self.index.writer(self.heap_size)?;
        index_writer.delete_all_documents()?;
        index_writer.commit()?;
        Ok(())
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
        paths: HashSet<&str>,
    ) -> anyhow::Result<(Docs<'_>, SnippetGenerator)> {
        let query_parser = QueryParser::for_index(&self.index, vec![self.fields.line]);
        let query = query_parser.parse_query(query)?;
        self.query(&query, limit, paths)
    }

    pub fn docs(&self, paths: HashSet<&str>) -> anyhow::Result<Docs<'_>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let searcher = reader.searcher();
        let docs = Self::glob(paths)
            .map(|p| -> anyhow::Result<_> {
                let path_term =
                    Term::from_field_text(self.fields.path, p.to_string_lossy().as_ref());
                let term_query = TermQuery::new(path_term.clone(), IndexRecordOption::Basic);
                let top_docs = searcher.search(&term_query, &TopDocs::with_limit(1))?;
                if let Some((_score, doc_address)) = top_docs.first() {
                    Ok(Some(Doc {
                        fields: &self.fields,
                        doc: searcher.doc(*doc_address)?,
                    }))
                } else {
                    Ok(None)
                }
            })
            .filter_map(Result::transpose)
            .collect::<Vec<_>>();
        Ok(box docs.into_iter())
    }

    fn query(
        &self,
        query: &dyn Query,
        limit: usize,
        paths: HashSet<&str>,
    ) -> anyhow::Result<(Docs<'_>, SnippetGenerator)> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let searcher = Arc::new(reader.searcher());
        let snippet_generator = SnippetGenerator::create(&searcher, query, self.fields.line)?;
        let path_set = Arc::new(Self::glob(paths).collect::<HashSet<PathBuf>>());
        let path_set_cpy = path_set.clone();
        let fields = Arc::new(self.fields.clone());
        let top_docs = searcher.search(
            query,
            &TopDocs::with_limit(limit).tweak_score(move |segment_reader: &SegmentReader| {
                let store_reader = segment_reader
                    .get_store_reader()
                    .expect("tweaking score needs store reader");
                let p_set = path_set_cpy.clone();
                let fields = fields.clone();
                move |doc_id, original_score| {
                    let doc = Doc {
                        fields: &fields,
                        doc: store_reader
                            .get(doc_id)
                            .expect("get document from store reader"),
                    };

                    let path: &Path = doc.path().unwrap().as_ref();
                    if p_set.contains(path) {
                        original_score
                    } else {
                        f32::MIN
                    }
                }
            }),
        )?;
        let docs = top_docs
            .into_iter()
            .map(move |(_, addr)| searcher.clone().doc(addr))
            .map(|d| {
                Ok(Doc {
                    fields: &self.fields,
                    doc: d?,
                })
            })
            .filter_map(move |d| {
                if let Ok(ref doc) = d {
                    let path: &Path = doc.path()?.as_ref();
                    if !path_set.contains(path) {
                        return None;
                    }
                }
                Some(d)
            });
        Ok((box docs, snippet_generator))
    }

    fn glob<'a>(paths: HashSet<&'a str>) -> impl 'a + ParallelIterator<Item = PathBuf> {
        paths
            .into_iter()
            .filter_map(|p| glob(p).ok())
            .flat_map(|paths| paths)
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
    }
}
