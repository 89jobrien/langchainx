mod document_loader;
pub use document_loader::*;

mod text_loader;
pub use text_loader::*;

mod csv_loader;
pub use csv_loader::*;

#[cfg(feature = "git")]
mod git_commit_loader;
#[cfg(feature = "git")]
pub use git_commit_loader::*;

mod pandoc_loader;
pub use pandoc_loader::*;

#[cfg(any(feature = "lopdf", feature = "pdf-extract"))]
mod pdf_loader;
#[cfg(any(feature = "lopdf", feature = "pdf-extract"))]
pub use pdf_loader::*;

mod html_loader;
pub use html_loader::*;

#[cfg(feature = "html-to-markdown")]
mod html_to_markdown_loader;
#[cfg(feature = "html-to-markdown")]
pub use html_to_markdown_loader::*;

mod json_loader;
pub use json_loader::*;

mod error;
pub use error::*;

mod dir_loader;
pub use dir_loader::*;

mod markdown_loader;
pub use markdown_loader::*;

#[cfg(feature = "sitemap")]
pub mod sitemap_loader;
#[cfg(feature = "sitemap")]
pub use sitemap_loader::*;

#[cfg(feature = "tree-sitter")]
mod source_code_loader;
#[cfg(feature = "tree-sitter")]
pub use source_code_loader::*;

#[cfg(feature = "rss")]
pub mod rss_loader;
#[cfg(feature = "rss")]
pub use rss_loader::*;
