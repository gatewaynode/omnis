//! JSON-RPC 2.0 framing: one request or notification per line in, one response per line out.

use serde_json::{Value, json};

/// Parse error.
pub const PARSE_ERROR: i64 = -32700;
/// Not a valid request object.
pub const INVALID_REQUEST: i64 = -32600;
/// Unknown method.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// Bad params.
pub const INVALID_PARAMS: i64 = -32602;
/// Anything else.
pub const INTERNAL_ERROR: i64 = -32603;

/// An inbound message. Notifications have no `id`.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The id to answer with, or `None` for a notification.
    pub id: Option<Value>,
    /// The method.
    pub method: String,
    /// The params, `Null` when absent.
    pub params: Value,
}

/// Parse one line. `Err` carries the id (when any) and the error code and message.
pub fn parse(line: &str) -> Result<Request, (Value, i64, String)> {
    let value: Value =
        serde_json::from_str(line).map_err(|e| (Value::Null, PARSE_ERROR, e.to_string()))?;
    let Some(object) = value.as_object() else {
        return Err((
            Value::Null,
            INVALID_REQUEST,
            "a request is an object".into(),
        ));
    };
    let id = object.get("id").cloned().filter(|id| !id.is_null());
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return Err((
            id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "missing method".into(),
        ));
    };
    Ok(Request {
        id,
        method: method.to_owned(),
        params: object.get("params").cloned().unwrap_or(Value::Null),
    })
}

/// A success response line.
#[must_use]
pub fn result(id: &Value, result: Value) -> String {
    line(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

/// An error response line.
#[must_use]
pub fn error(id: &Value, code: i64, message: &str) -> String {
    line(json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}))
}

/// An error response line with a `data` member.
#[must_use]
pub fn error_data(id: &Value, code: i64, message: &str, data: Value) -> String {
    line(
        json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message, "data": data}}),
    )
}

fn line(value: Value) -> String {
    let mut text = value.to_string();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_notifications_and_errors() {
        let request = parse(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).unwrap();
        assert_eq!(
            (request.id, request.method.as_str()),
            (Some(json!(1)), "ping")
        );
        assert_eq!(request.params, Value::Null);
        let note = parse(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).unwrap();
        assert_eq!(note.id, None);
        assert_eq!(parse("{").unwrap_err().1, PARSE_ERROR);
        assert_eq!(parse("[]").unwrap_err().1, INVALID_REQUEST);
        let (id, code, _) = parse(r#"{"id": "k"}"#).unwrap_err();
        assert_eq!((id, code), (json!("k"), INVALID_REQUEST));
        assert_eq!(
            result(&json!(1), json!({})),
            "{\"id\":1,\"jsonrpc\":\"2.0\",\"result\":{}}\n"
        );
        assert!(error(&Value::Null, METHOD_NOT_FOUND, "x").contains("-32601"));
    }
}
