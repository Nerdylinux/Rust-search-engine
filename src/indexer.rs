use std::path::Path;
use tantivy::schema::*;
use tantivy::{Index, doc};

pub struct SearchIndex {
    index: Index,
    schema: Schema,
}

impl SearchIndex {
    pub fn create_or_open(path: &str) -> tantivy::Result<Self> {
        // Define schema with fuzzy search support
        let mut schema_builder = Schema::builder();

        // TEXT for full-text search with tokenization
        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("en_stem")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        schema_builder.add_text_field("title", text_options.clone());
        schema_builder.add_text_field("body", TEXT);
        schema_builder.add_text_field("url", STRING | STORED);

        let schema = schema_builder.build();

        // Create or open index on disk
        let index = if Path::new(path).exists() {
            Index::open_in_dir(path)?
        } else {
            std::fs::create_dir_all(path)?;
            Index::create_in_dir(path, schema.clone())?
        };

        Ok(SearchIndex { index, schema })
    }

    pub fn add_document(&self, title: &str, body: &str, url: &str) -> tantivy::Result<()> {
        let mut writer = self.index.writer(50_000_000)?;

        let title_field = self.schema.get_field("title").unwrap();
        let body_field = self.schema.get_field("body").unwrap();
        let url_field = self.schema.get_field("url").unwrap();

        writer.add_document(doc!(
            title_field => title,
            body_field => body,
            url_field => url
        ))?;

        writer.commit()?;
        Ok(())
    }

    pub fn search(
        &self,
        query_str: &str,
        limit: usize,
    ) -> tantivy::Result<Vec<(String, String, f32)>> {
        use tantivy::collector::TopDocs;
        use tantivy::query::QueryParser;

        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let title_field = self.schema.get_field("title").unwrap();
        let body_field = self.schema.get_field("body").unwrap();
        let url_field = self.schema.get_field("url").unwrap();

        let query_parser = QueryParser::for_index(&self.index, vec![title_field, body_field]);
        let query = query_parser.parse_query(query_str)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let title = doc
                .get_all(title_field)
                .next()
                .and_then(|v| v.as_str())
                .unwrap_or("No title");
            let url = doc
                .get_all(url_field)
                .next()
                .and_then(|v| v.as_str())
                .unwrap_or("");

            results.push((title.to_string(), url.to_string(), score));
        }

        Ok(results)
    }

    pub fn fuzzy_search(
        &self,
        query_str: &str,
        limit: usize,
        distance: u8,
    ) -> tantivy::Result<Vec<(String, String, f32)>> {
        use tantivy::Term;
        use tantivy::collector::TopDocs;
        use tantivy::query::FuzzyTermQuery;

        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let title_field = self.schema.get_field("title").unwrap();
        let body_field = self.schema.get_field("body").unwrap();
        let url_field = self.schema.get_field("url").unwrap();

        // Create fuzzy queries for both title and body
        let title_term = Term::from_field_text(title_field, query_str);
        let body_term = Term::from_field_text(body_field, query_str);

        let title_fuzzy = FuzzyTermQuery::new(title_term, distance, true);
        let body_fuzzy = FuzzyTermQuery::new(body_term, distance, true);

        // Combine queries with OR logic
        use tantivy::query::BooleanQuery;
        use tantivy::query::Occur;

        let query = BooleanQuery::new(vec![
            (Occur::Should, Box::new(title_fuzzy)),
            (Occur::Should, Box::new(body_fuzzy)),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let title = doc
                .get_all(title_field)
                .next()
                .and_then(|v| v.as_str())
                .unwrap_or("No title");
            let url = doc
                .get_all(url_field)
                .next()
                .and_then(|v| v.as_str())
                .unwrap_or("");

            results.push((title.to_string(), url.to_string(), score));
        }

        Ok(results)
    }

    pub fn count_documents(&self) -> tantivy::Result<usize> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        Ok(searcher.num_docs() as usize)
    }
}
