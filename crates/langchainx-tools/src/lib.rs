#![allow(clippy::module_inception)]

pub use langchainx_core::tools::{Tool, ToolError};

pub mod wolfram;
pub use wolfram::Wolfram;

pub mod scraper;
pub use scraper::WebScrapper;

pub mod sql;
pub use sql::{Dialect, Engine, SQLDatabase, SQLDatabaseBuilder};

pub mod duckduckgo;
pub use duckduckgo::{DuckDuckGoSearchResults, SearchResult};

pub mod serpapi;
pub use serpapi::SerpApi;

pub mod command_executor;
pub use command_executor::CommandExecutor;

pub mod text2speech;
pub use text2speech::{SpeechStorage, Text2SpeechOpenAI};

// Placeholder modules — not yet implemented. Tracking: issue #39.
// Hidden from public docs until implementation is complete.
#[doc(hidden)]
pub mod confluence;
#[doc(hidden)]
pub mod google;
#[doc(hidden)]
pub mod jira;

pub mod minibox;
pub use minibox::MiniboxTool;
