//! Common GraphQL types

use async_graphql::{Scalar, ScalarType, Value};
use chrono::{DateTime as ChronoDateTime, Utc};

/// DateTime scalar
#[derive(Debug, Clone)]
pub struct DateTime(pub ChronoDateTime<Utc>);

#[Scalar]
impl ScalarType for DateTime {
    fn parse(value: Value) -> async_graphql::InputValueResult<Self> {
        if let Value::String(s) = value {
            Ok(DateTime(
                ChronoDateTime::parse_from_rfc3339(&s)
                    .map_err(|e| format!("Invalid DateTime: {}", e))?
                    .with_timezone(&Utc),
            ))
        } else {
            Err("Expected string for DateTime".into())
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_rfc3339())
    }
}

/// File upload scalar
#[derive(Debug, Clone)]
pub struct Upload {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_to_value() {
        let dt = DateTime(Utc::now());
        let value = dt.to_value();
        assert!(matches!(value, Value::String(_)));
    }
}
