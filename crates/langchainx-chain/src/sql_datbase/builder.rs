//! Builder for the PostgreSQL question-answering chain.
use std::sync::Arc;

use crate::{
    chain::{
        ChainError, DEFAULT_OUTPUT_KEY, llm_chain::LLMChainBuilder, options::ChainCallOptions,
    },
    language_models::llm::{DynLLM, IntoArcLLM},
    output_parsers::OutputParser,
    prompt::HumanMessagePromptTemplate,
    template_jinja2,
};
use langchainx_tools::SQLDatabase;

use super::{
    STOP_WORD,
    chain::SQLDatabaseChain,
    prompt::{DEFAULT_SQLSUFFIX, DEFAULT_SQLTEMPLATE},
};

/// Configures the model, database, row limit, parser, and output options for an [`SQLDatabaseChain`].
pub struct SQLDatabaseChainBuilder {
    llm: Option<Arc<dyn DynLLM>>,
    options: Option<ChainCallOptions>,
    top_k: Option<usize>,
    database: Option<SQLDatabase>,
    output_key: Option<String>,
    output_parser: Option<Box<dyn OutputParser>>,
}

#[allow(clippy::new_without_default)] // Builder pattern
impl SQLDatabaseChainBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self {
            llm: None,
            options: None,
            top_k: None,
            database: None,
            output_key: None,
            output_parser: None,
        }
    }

    /// Sets the language model that generates SQL and summarizes query results.
    pub fn llm<L: IntoArcLLM>(mut self, llm: L) -> Self {
        self.llm = Some(llm.into_arc_llm());
        self
    }

    /// Sets the structured-output key for generated text.
    pub fn output_key<S: Into<String>>(mut self, output_key: S) -> Self {
        self.output_key = Some(output_key.into());
        self
    }

    /// Sets the parser applied to language-model generations.
    pub fn output_parser<P: Into<Box<dyn OutputParser>>>(mut self, output_parser: P) -> Self {
        self.output_parser = Some(output_parser.into());
        self
    }

    /// Sets model call options; the SQL-result marker is always added as a stop sequence.
    pub fn options(mut self, options: ChainCallOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Sets the maximum row count requested in generated SQL.
    pub fn top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Sets the database whose schema is described and whose queries are executed.
    pub fn database(mut self, database: SQLDatabase) -> Self {
        self.database = Some(database);
        self
    }

    /// Builds the chain, requiring a model, row limit, and database.
    pub fn build(self) -> Result<SQLDatabaseChain, ChainError> {
        let llm = self
            .llm
            .ok_or_else(|| ChainError::MissingObject("LLM must be set".into()))?;
        let top_k = self
            .top_k
            .ok_or_else(|| ChainError::MissingObject("Top K must be set".into()))?;
        let database = self
            .database
            .ok_or_else(|| ChainError::MissingObject("Database must be set".into()))?;

        let prompt = HumanMessagePromptTemplate::new(template_jinja2!(
            format!("{}{}", DEFAULT_SQLTEMPLATE, DEFAULT_SQLSUFFIX),
            "dialect",
            "table_info",
            "top_k",
            "input"
        ));

        let llm_chain = {
            let mut builder = LLMChainBuilder::new()
                .prompt(prompt)
                .output_key(self.output_key.unwrap_or_else(|| DEFAULT_OUTPUT_KEY.into()))
                .llm(llm);

            let mut options = self.options.unwrap_or_default();
            options = options.with_stop_words(vec![STOP_WORD.to_string()]);
            builder = builder.options(options);

            if let Some(output_parser) = self.output_parser {
                builder = builder.output_parser(output_parser);
            }

            builder.build()?
        };

        Ok(SQLDatabaseChain {
            llmchain: llm_chain,
            top_k,
            database,
        })
    }
}
