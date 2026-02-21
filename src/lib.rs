//! # pleme-graphql-helpers
//!
//! GraphQL utilities library for Pleme platform services.
//!
//! ## Features
//!
//! - **Cursor Pagination** - Relay-style cursor pagination
//! - **Federation Helpers** - Apollo Federation v2 utilities
//! - **Common Types** - Reusable GraphQL types
//! - **DataLoader** - Batch loading for N+1 prevention
//! - **Auth Middleware** - JWT and context extraction for GraphQL handlers
//!
//! ## Usage
//!
//! ```rust
//! use pleme_graphql_helpers::pagination::{Connection, Edge};
//!
//! // Create paginated response
//! let connection = Connection::new(items, has_next_page, has_previous_page);
//! ```

pub mod pagination;
pub mod federation;
pub mod types;
pub mod dataloaders;
pub mod auth;

pub use pagination::{Connection, Edge, PageInfo, CursorCodec, PaginationInput};
pub use federation::EntityResolver;
pub use types::{DateTime, Upload};
pub use dataloaders::{BatchLoader, DataLoader};
pub use auth::{graphql_handler, extract_user_id, extract_company_id, extract_authz};

use thiserror::Error;

/// GraphQL errors
#[derive(Error, Debug)]
pub enum GraphQLError {
    #[error("Invalid cursor: {0}")]
    InvalidCursor(String),

    #[error("Pagination error: {0}")]
    PaginationError(String),

    #[error("Federation error: {0}")]
    FederationError(String),
}

/// Result type for GraphQL operations
pub type Result<T> = std::result::Result<T, GraphQLError>;
