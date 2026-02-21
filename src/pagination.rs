//! Relay-style cursor pagination

use async_graphql::{Object, SimpleObject, InputObject};
use serde::{Serialize, Deserialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Page information
#[derive(SimpleObject, Debug, Clone)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

/// Edge in a connection
#[derive(Debug, Clone)]
pub struct Edge<T> {
    pub cursor: String,
    pub node: T,
}

#[Object]
impl<T: async_graphql::OutputType> Edge<T> {
    async fn cursor(&self) -> &str {
        &self.cursor
    }

    async fn node(&self) -> &T {
        &self.node
    }
}

/// Connection (paginated result)
#[derive(Debug, Clone)]
pub struct Connection<T> {
    pub edges: Vec<Edge<T>>,
    pub page_info: PageInfo,
}

#[Object]
impl<T: async_graphql::OutputType> Connection<T> {
    async fn edges(&self) -> &[Edge<T>] {
        &self.edges
    }

    async fn page_info(&self) -> &PageInfo {
        &self.page_info
    }
}

impl<T> Connection<T> {
    /// Create new connection
    pub fn new(items: Vec<T>, has_next: bool, has_previous: bool) -> Self
    where
        T: Serialize,
    {
        let edges: Vec<Edge<T>> = items
            .into_iter()
            .enumerate()
            .map(|(idx, node)| {
                let cursor = CursorCodec::encode(&idx.to_string());
                Edge { cursor, node }
            })
            .collect();

        let start_cursor = edges.first().map(|e| e.cursor.clone());
        let end_cursor = edges.last().map(|e| e.cursor.clone());

        Self {
            edges,
            page_info: PageInfo {
                has_next_page: has_next,
                has_previous_page: has_previous,
                start_cursor,
                end_cursor,
            },
        }
    }

    /// Create empty connection
    pub fn empty() -> Self {
        Self {
            edges: Vec::new(),
            page_info: PageInfo {
                has_next_page: false,
                has_previous_page: false,
                start_cursor: None,
                end_cursor: None,
            },
        }
    }
}

/// Cursor encoding/decoding
pub struct CursorCodec;

impl CursorCodec {
    /// Encode cursor to base64
    pub fn encode(value: &str) -> String {
        BASE64.encode(value.as_bytes())
    }

    /// Decode cursor from base64
    pub fn decode(cursor: &str) -> crate::Result<String> {
        let bytes = BASE64
            .decode(cursor.as_bytes())
            .map_err(|e| crate::GraphQLError::InvalidCursor(e.to_string()))?;
        String::from_utf8(bytes)
            .map_err(|e| crate::GraphQLError::InvalidCursor(e.to_string()))
    }

    /// Encode structured cursor (e.g., timestamp + ID)
    pub fn encode_structured<T: Serialize>(value: &T) -> crate::Result<String> {
        let json = serde_json::to_string(value)
            .map_err(|e| crate::GraphQLError::InvalidCursor(e.to_string()))?;
        Ok(BASE64.encode(json.as_bytes()))
    }

    /// Decode structured cursor
    pub fn decode_structured<T: for<'de> Deserialize<'de>>(cursor: &str) -> crate::Result<T> {
        let bytes = BASE64
            .decode(cursor.as_bytes())
            .map_err(|e| crate::GraphQLError::InvalidCursor(e.to_string()))?;
        let json = String::from_utf8(bytes)
            .map_err(|e| crate::GraphQLError::InvalidCursor(e.to_string()))?;
        serde_json::from_str(&json)
            .map_err(|e| crate::GraphQLError::InvalidCursor(e.to_string()))
    }
}

/// Pagination input for GraphQL queries
///
/// Follows the Relay Cursor Connections Specification:
/// https://relay.dev/graphql/connections.htm
#[derive(InputObject, Debug, Clone)]
pub struct PaginationInput {
    /// Number of items to return (forward pagination)
    pub first: Option<i32>,

    /// Cursor to start from (forward pagination)
    pub after: Option<String>,

    /// Number of items to return (backward pagination)
    pub last: Option<i32>,

    /// Cursor to start from (backward pagination)
    pub before: Option<String>,
}

impl PaginationInput {
    /// Validate pagination input
    pub fn validate(&self) -> crate::Result<()> {
        if self.first.is_some() && self.last.is_some() {
            return Err(crate::GraphQLError::PaginationError(
                "Cannot specify both 'first' and 'last'".to_string(),
            ));
        }

        if let Some(first) = self.first {
            if first < 0 {
                return Err(crate::GraphQLError::PaginationError(
                    "'first' must be non-negative".to_string(),
                ));
            }
            if first > 100 {
                return Err(crate::GraphQLError::PaginationError(
                    "'first' cannot exceed 100".to_string(),
                ));
            }
        }

        if let Some(last) = self.last {
            if last < 0 {
                return Err(crate::GraphQLError::PaginationError(
                    "'last' must be non-negative".to_string(),
                ));
            }
            if last > 100 {
                return Err(crate::GraphQLError::PaginationError(
                    "'last' cannot exceed 100".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Get limit for database query
    pub fn limit(&self) -> i32 {
        self.first
            .or(self.last)
            .unwrap_or(20)
            .min(100) // Cap at 100
    }

    /// Check if forward pagination
    pub fn is_forward(&self) -> bool {
        self.first.is_some() || self.after.is_some()
    }

    /// Check if backward pagination
    pub fn is_backward(&self) -> bool {
        self.last.is_some() || self.before.is_some()
    }
}

impl Default for PaginationInput {
    fn default() -> Self {
        Self {
            first: Some(20),
            after: None,
            last: None,
            before: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Item {
        id: String,
    }

    #[test]
    fn test_cursor_codec() {
        let original = "test-cursor";
        let encoded = CursorCodec::encode(original);
        let decoded = CursorCodec::decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_connection_creation() {
        let items = vec![
            Item { id: "1".to_string() },
            Item { id: "2".to_string() },
        ];
        let conn = Connection::new(items, true, false);
        assert_eq!(conn.edges.len(), 2);
        assert!(conn.page_info.has_next_page);
        assert!(!conn.page_info.has_previous_page);
    }
}
