use crate::{DirLoaderOptions, Loader, LoaderError, find_files_with_extension, process_doc_stream};
use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;

use std::fs::File;
use std::io::Read;
use std::pin::Pin;

use super::{LanguageParser, LanguageParserOptions, get_language_by_filename};

#[derive(Debug, Clone)]
pub struct SourceCodeLoader {
    file_path: Option<String>,
    string_input: Option<String>,
    dir_loader_options: DirLoaderOptions,
    parser_option: LanguageParserOptions,
}

impl SourceCodeLoader {
    pub fn from_string<S: Into<String>>(input: S) -> Self {
        Self {
            string_input: Some(input.into()),
            file_path: None,
            parser_option: LanguageParserOptions::default(),
            dir_loader_options: DirLoaderOptions::default(),
        }
    }
}

impl SourceCodeLoader {
    pub fn from_path<S: Into<String>>(path: S) -> Self {
        Self {
            file_path: Some(path.into()),
            string_input: None,
            parser_option: LanguageParserOptions::default(),
            dir_loader_options: DirLoaderOptions::default(),
        }
    }
}

impl SourceCodeLoader {
    pub fn with_parser_option(mut self, parser_option: LanguageParserOptions) -> Self {
        self.parser_option = parser_option;
        self
    }

    pub fn with_dir_loader_options(mut self, dir_loader_options: DirLoaderOptions) -> Self {
        self.dir_loader_options = dir_loader_options;
        self
    }
}

#[async_trait]
impl Loader for SourceCodeLoader {
    async fn load(
        mut self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let string_input = self.string_input.clone();
        let file_path = self.file_path.clone();

        if let Some(file_path) = file_path {
            let files =
                find_files_with_extension(file_path.as_str(), &self.dir_loader_options).await?;
            let stream = stream! {
                for filename in files {
                    let mut file = match File::open(&filename) {
                        Ok(file) => file,
                        Err(e) => {
                            yield Err(LoaderError::OtherError(format!("Error opening file: {:?}", e)));
                            continue;
                        }
                    };
                    let mut content = String::new();
                    file.read_to_string(&mut content).unwrap();
                    let language = get_language_by_filename(&filename);
                    let mut parser = LanguageParser::from_language(language).with_parser_option(self.parser_option.clone());
                    let docs = parser.parse_code(&content);
                    for doc in docs {
                        yield Ok(doc);
                    }
                }
            };

            return Ok(Box::pin(stream));
        } else if let Some(content) = string_input {
            let language = self.parser_option.language.clone();
            let stream = stream! {
                    let mut parser = LanguageParser::from_language(language).with_parser_option(self.parser_option.clone());
                    let docs = parser.parse_code(&content);
                    for doc in docs {
                        yield Ok(doc);
                    }
            };

            return Ok(Box::pin(stream));
        }
        Err(LoaderError::OtherError(
            "No file path or string input provided".to_string(),
        ))
    }

    async fn load_and_split<TS: TextSplitter + 'static>(
        mut self,
        splitter: TS,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let doc_stream = self.load().await?;
        let stream = process_doc_stream(doc_stream, splitter).await;
        Ok(Box::pin(stream))
    }
}
