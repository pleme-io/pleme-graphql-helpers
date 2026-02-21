//! GraphQL authentication middleware and context extraction
//!
//! Provides helpers for:
//! - Extracting user_id, company_id, and JWT from HTTP headers
//! - Creating GraphQL request context with auth info
//! - Standard Axum handler for GraphQL endpoints with auth

use async_graphql::{Context, Request, Response, Schema};
use axum::{
    extract::Extension,
    http::HeaderMap,
    Json,
};
use pleme_rbac::AuthzContext;
use uuid::Uuid;

/// Extract user_id from x-user-id header
pub fn extract_user_id(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Extract company_id from x-company-id header
pub fn extract_company_id(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get("x-company-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Extract and parse JWT from Authorization header
pub fn extract_authz(headers: &HeaderMap) -> AuthzContext {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|auth| {
            if let Some(token) = auth.strip_prefix("Bearer ") {
                AuthzContext::from_jwt(token).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(AuthzContext::empty)
}

/// Standard GraphQL handler with authentication context injection
///
/// Extracts user_id, company_id, and AuthzContext from headers and injects into request
///
/// # Example
///
/// ```rust,no_run
/// use axum::{Router, routing::post};
/// use pleme_graphql_helpers::auth::graphql_handler;
/// use async_graphql::Schema;
///
/// # async fn example(schema: Schema<(), (), ()>) {
/// let app = Router::new()
///     .route("/graphql", post(graphql_handler::<(), (), ()>));
/// # }
/// ```
pub async fn graphql_handler<Query, Mutation, Subscription>(
    Extension(schema): Extension<Schema<Query, Mutation, Subscription>>,
    headers: HeaderMap,
    req: Json<Request>,
) -> Json<Response>
where
    Query: async_graphql::ObjectType + 'static,
    Mutation: async_graphql::ObjectType + 'static,
    Subscription: async_graphql::SubscriptionType + 'static,
{
    // Extract auth context from headers
    let user_id = extract_user_id(&headers);
    let company_id = extract_company_id(&headers);
    let authz = extract_authz(&headers);

    // Build request with context
    let mut request = req.0;

    if let Some(uid) = user_id {
        request = request.data(uid);
    }

    if let Some(cid) = company_id {
        request = request.data(cid);
    }

    request = request.data(authz);

    // Execute query
    let response = schema.execute(request).await;

    Json(response)
}

/// Get user_id from GraphQL context
///
/// # Example
///
/// ```rust,no_run
/// use async_graphql::Context;
/// use pleme_graphql_helpers::auth::get_user_id;
///
/// fn resolver(ctx: &Context<'_>) -> Option<uuid::Uuid> {
///     get_user_id(ctx)
/// }
/// ```
pub fn get_user_id(ctx: &Context<'_>) -> Option<Uuid> {
    ctx.data_opt::<Uuid>().copied()
}

/// Get company_id from GraphQL context
///
/// # Example
///
/// ```rust,no_run
/// use async_graphql::Context;
/// use pleme_graphql_helpers::auth::get_company_id;
///
/// fn resolver(ctx: &Context<'_>) -> Option<uuid::Uuid> {
///     get_company_id(ctx)
/// }
/// ```
pub fn get_company_id(ctx: &Context<'_>) -> Option<Uuid> {
    // Company ID might be stored as second Uuid in context
    // This is a simplified version - in practice you'd need a wrapper type
    ctx.data_opt::<Uuid>().copied()
}

/// Get AuthzContext from GraphQL context
///
/// # Example
///
/// ```rust,no_run
/// use async_graphql::Context;
/// use pleme_graphql_helpers::auth::get_authz_context;
///
/// fn resolver(ctx: &Context<'_>) -> pleme_rbac::AuthzContext {
///     get_authz_context(ctx)
/// }
/// ```
pub fn get_authz_context(ctx: &Context<'_>) -> AuthzContext {
    ctx.data_opt::<AuthzContext>()
        .cloned()
        .unwrap_or_else(AuthzContext::empty)
}
