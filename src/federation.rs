//! Apollo Federation v2 utilities

use async_trait::async_trait;

/// Entity resolver trait for Apollo Federation
#[async_trait]
pub trait EntityResolver: Send + Sync {
    /// Resolve entity by key
    async fn resolve_reference(&self, key: &str) -> Option<String>;
}

// Federation helper macros would go here
// For now, keeping it minimal
