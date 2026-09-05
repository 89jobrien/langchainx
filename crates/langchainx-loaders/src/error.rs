//! Errors produced while loading or splitting documents.
use std::{io, string::FromUtf8Error};

use thiserror::Error;

use langchainx_text_splitter::TextSplitterError;

#[derive(Error, Debug)]
/// Errors produced by document loaders.
pub enum LoaderError {
    #[error("Error loading document: {0}")]
    /// A loader could not retrieve or decode a document.
    LoadDocumentError(String),

    #[error("{0}")]
    /// A document could not be split.
    TextSplitterError(#[from] TextSplitterError),

    #[error(transparent)]
    /// An I/O operation failed.
    IOError(#[from] io::Error),

    #[error(transparent)]
    /// Loaded bytes were not valid UTF-8.
    FromUtf8Error(#[from] FromUtf8Error),

    #[error(transparent)]
    /// CSV parsing failed.
    CSVError(#[from] csv::Error),

    #[cfg(feature = "lopdf")]
    #[cfg(not(feature = "pdf-extract"))]
    #[error(transparent)]
    /// PDF parsing with `lopdf` failed.
    LoPdfError(#[from] lopdf::Error),

    #[cfg(feature = "pdf-extract")]
    #[error(transparent)]
    /// PDF parsing with `pdf-extract` failed.
    PdfExtractError(#[from] pdf_extract::Error),

    #[cfg(feature = "pdf-extract")]
    #[error(transparent)]
    /// Writing extracted PDF text failed.
    PdfExtractOutputError(#[from] pdf_extract::OutputError),

    #[error(transparent)]
    /// HTML readability extraction failed.
    ReadabilityError(#[from] readability::error::Error),

    #[error(transparent)]
    /// A spawned Tokio task failed.
    JoinError(#[from] tokio::task::JoinError),

    #[cfg(feature = "git")]
    #[error(transparent)]
    /// Git repository discovery failed.
    DiscoveryError(#[from] gix::discover::Error),

    #[error("Error: {0}")]
    /// A loader-specific error not represented by another variant.
    OtherError(String),
}
