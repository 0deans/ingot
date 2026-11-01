pub mod error;
pub mod models;
pub mod service;

#[cfg(test)]
mod tests;

pub use error::NewsError;
pub use models::{NewsArticle, NewsFeedResponse, PatchNoteDetails, PatchNoteSummary};
pub use service::NewsService;
