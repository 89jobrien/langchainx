use std::{io::Read, path::Path, pin::Pin};

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;
use pdf_extract::{output_doc, PlainTextOutput};

use crate::{process_doc_stream, Loader, LoaderError};

#[derive(Debug, Clone)]
pub struct PdfExtractLoader {
    document: pdf_extract::Document,
}

impl PdfExtractLoader {
    pub fn new<R: Read>(reader: R) -> Result<Self, LoaderError> {
        let document = pdf_extract::Document::load_from(reader)?;
        Ok(Self { document })
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoaderError> {
        let document = pdf_extract::Document::load(path)?;
        Ok(Self { document })
    }
}

#[async_trait]
impl Loader for PdfExtractLoader {
    async fn load(
        mut self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let mut buffer: Vec<u8> = Vec::new();
        let mut output = PlainTextOutput::new(&mut buffer as &mut dyn std::io::Write);
        output_doc(&self.document, &mut output)?;
        let doc = Document::new(String::from_utf8(buffer)?);

        let stream = stream! {
            yield Ok(doc);
        };

        Ok(Box::pin(stream))
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
