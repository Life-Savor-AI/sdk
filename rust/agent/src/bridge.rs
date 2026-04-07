//! Bridge protocol types shared between the agent and SDK.
//!
//! This module defines the request/response types used by the System Component
//! Bridge to route skill requests to system components. Only the data types
//! are included here — dispatch logic (`SystemComponentBridge`), rate-limiter
//! runtime state (`BridgeRateLimiter`), and permission enforcement stay in the
//! agent crate.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// BridgeRequest
// ---------------------------------------------------------------------------

/// A request from a sandboxed skill to invoke a system component operation.
///
/// For MCP-based skills the tool name is parsed into `component` + `operation`
/// (e.g. `system.tts.synthesize` → component=`tts`, operation=`synthesize`).
///
/// For JSON stdin/stdout skills the `system_call` operation type carries the
/// same fields in the JSON payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BridgeRequest {
    /// Target system component name (e.g. `"tts"`, `"file_storage"`, `"messaging"`).
    pub component: String,
    /// Operation to invoke on the component (e.g. `"synthesize"`, `"write"`, `"send"`).
    pub operation: String,
    /// Operation parameters as a JSON value.
    pub params: Value,
    /// Identity of the requesting skill.
    pub skill_id: String,
    /// Optional correlation ID from the outer request context.
    pub correlation_id: Option<String>,
}

// ---------------------------------------------------------------------------
// BridgeResponse
// ---------------------------------------------------------------------------

/// Response returned to the skill after a bridge call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BridgeResponse {
    /// Whether the call succeeded.
    pub success: bool,
    /// Result payload (component-specific).
    pub result: Value,
    /// Error details when `success` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BridgeError>,
}

impl BridgeResponse {
    /// Create a successful response.
    pub fn ok(result: Value) -> Self {
        Self {
            success: true,
            result,
            error: None,
        }
    }

    /// Create an error response.
    pub fn err(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            result: Value::Null,
            error: Some(BridgeError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// BridgeError
// ---------------------------------------------------------------------------

/// Structured error returned by the bridge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BridgeError {
    /// Machine-readable error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

// ---------------------------------------------------------------------------
// SystemCallRequest
// ---------------------------------------------------------------------------

/// Envelope for `system_call` operations over JSON stdin/stdout protocol.
///
/// Skills using the JSON stdin/stdout protocol send this as the request
/// payload when `operation_type` is `"system_call"`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemCallRequest {
    /// Must be `"system_call"`.
    pub operation_type: String,
    /// Target system component.
    pub component: String,
    /// Operation name.
    pub operation: String,
    /// Operation parameters.
    #[serde(default)]
    pub params: Value,
}

// ---------------------------------------------------------------------------
// SystemCallResponse
// ---------------------------------------------------------------------------

/// Response envelope for `system_call` over JSON stdin/stdout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemCallResponse {
    /// Echoed operation type.
    pub operation_type: String,
    /// Whether the call succeeded.
    pub success: bool,
    /// Result payload.
    pub result: Value,
    /// Error details when `success` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BridgeError>,
}

impl From<BridgeResponse> for SystemCallResponse {
    fn from(resp: BridgeResponse) -> Self {
        Self {
            operation_type: "system_call".to_string(),
            success: resp.success,
            result: resp.result,
            error: resp.error,
        }
    }
}

// ---------------------------------------------------------------------------
// BridgeRateLimit
// ---------------------------------------------------------------------------

/// Per-skill, per-component rate limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BridgeRateLimit {
    /// Maximum requests allowed in the window.
    pub max_requests: u32,
    /// Time window for the rate limit (in seconds for serialization).
    #[serde(with = "duration_secs")]
    pub window: Duration,
}

impl Default for BridgeRateLimit {
    fn default() -> Self {
        Self {
            max_requests: 60,
            window: Duration::from_secs(60),
        }
    }
}

/// Serde helper to serialize/deserialize `Duration` as seconds (u64).
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_secs().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // -- Unit tests -------------------------------------------------------

    #[test]
    fn bridge_response_ok_helper() {
        let resp = BridgeResponse::ok(serde_json::json!({"key": "value"}));
        assert!(resp.success);
        assert!(resp.error.is_none());
        assert_eq!(resp.result, serde_json::json!({"key": "value"}));
    }

    #[test]
    fn bridge_response_err_helper() {
        let resp = BridgeResponse::err("ERR_CODE", "something failed");
        assert!(!resp.success);
        assert_eq!(resp.result, Value::Null);
        let err = resp.error.unwrap();
        assert_eq!(err.code, "ERR_CODE");
        assert_eq!(err.message, "something failed");
    }

    #[test]
    fn system_call_response_from_bridge_response() {
        let bridge = BridgeResponse::ok(serde_json::json!(42));
        let scr: SystemCallResponse = bridge.into();
        assert_eq!(scr.operation_type, "system_call");
        assert!(scr.success);
        assert_eq!(scr.result, serde_json::json!(42));
        assert!(scr.error.is_none());
    }

    #[test]
    fn bridge_rate_limit_default() {
        let limit = BridgeRateLimit::default();
        assert_eq!(limit.max_requests, 60);
        assert_eq!(limit.window, Duration::from_secs(60));
    }

    #[test]
    fn bridge_request_serde_unit() {
        let req = BridgeRequest {
            component: "tts".into(),
            operation: "synthesize".into(),
            params: serde_json::json!({"text": "hello"}),
            skill_id: "skill-1".into(),
            correlation_id: Some("corr-123".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: BridgeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn bridge_error_skip_serializing_none() {
        let resp = BridgeResponse::ok(Value::Null);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("error"));
    }

    // -- Proptest strategies ----------------------------------------------

    fn arb_json_value() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(|n| Value::Number(n.into())),
            "\\w{0,20}".prop_map(|s| Value::String(s)),
        ]
    }

    fn arb_bridge_request() -> impl Strategy<Value = BridgeRequest> {
        (
            "\\w{1,20}",
            "\\w{1,20}",
            arb_json_value(),
            "\\w{1,20}",
            proptest::option::of("\\w{1,30}"),
        )
            .prop_map(|(component, operation, params, skill_id, correlation_id)| {
                BridgeRequest {
                    component,
                    operation,
                    params,
                    skill_id,
                    correlation_id,
                }
            })
    }

    fn arb_bridge_error() -> impl Strategy<Value = BridgeError> {
        ("\\w{1,20}", "\\w{0,50}").prop_map(|(code, message)| BridgeError { code, message })
    }

    fn arb_bridge_response() -> impl Strategy<Value = BridgeResponse> {
        (any::<bool>(), arb_json_value(), proptest::option::of(arb_bridge_error())).prop_map(
            |(success, result, error)| BridgeResponse {
                success,
                result,
                error,
            },
        )
    }

    fn arb_system_call_request() -> impl Strategy<Value = SystemCallRequest> {
        ("\\w{1,20}", "\\w{1,20}", "\\w{1,20}", arb_json_value()).prop_map(
            |(operation_type, component, operation, params)| SystemCallRequest {
                operation_type,
                component,
                operation,
                params,
            },
        )
    }

    fn arb_system_call_response() -> impl Strategy<Value = SystemCallResponse> {
        (
            "\\w{1,20}",
            any::<bool>(),
            arb_json_value(),
            proptest::option::of(arb_bridge_error()),
        )
            .prop_map(|(operation_type, success, result, error)| {
                SystemCallResponse {
                    operation_type,
                    success,
                    result,
                    error,
                }
            })
    }

    fn arb_bridge_rate_limit() -> impl Strategy<Value = BridgeRateLimit> {
        (any::<u32>(), 1u64..=86400u64).prop_map(|(max_requests, secs)| BridgeRateLimit {
            max_requests,
            window: Duration::from_secs(secs),
        })
    }

    // -- Property tests ---------------------------------------------------

    proptest! {
        /// Property 1: Serde JSON round-trip for bridge protocol types
        ///
        /// **Validates: Requirements 3.2, 13.1**
        ///
        /// For any valid `BridgeRequest`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_bridge_request(req in arb_bridge_request()) {
            let json = serde_json::to_string(&req).unwrap();
            let back: BridgeRequest = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, req);
        }

        /// Property 1: Serde JSON round-trip for bridge protocol types
        ///
        /// **Validates: Requirements 3.2, 13.1**
        ///
        /// For any valid `BridgeResponse`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_bridge_response(resp in arb_bridge_response()) {
            let json = serde_json::to_string(&resp).unwrap();
            let back: BridgeResponse = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, resp);
        }

        /// Property 1: Serde JSON round-trip for bridge protocol types
        ///
        /// **Validates: Requirements 3.2, 13.1**
        ///
        /// For any valid `BridgeError`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_bridge_error(err in arb_bridge_error()) {
            let json = serde_json::to_string(&err).unwrap();
            let back: BridgeError = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, err);
        }

        /// Property 1: Serde JSON round-trip for bridge protocol types
        ///
        /// **Validates: Requirements 3.2, 13.1**
        ///
        /// For any valid `SystemCallRequest`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_system_call_request(req in arb_system_call_request()) {
            let json = serde_json::to_string(&req).unwrap();
            let back: SystemCallRequest = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, req);
        }

        /// Property 1: Serde JSON round-trip for bridge protocol types
        ///
        /// **Validates: Requirements 3.2, 13.1**
        ///
        /// For any valid `SystemCallResponse`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_system_call_response(resp in arb_system_call_response()) {
            let json = serde_json::to_string(&resp).unwrap();
            let back: SystemCallResponse = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, resp);
        }

        /// Property 1: Serde JSON round-trip for bridge protocol types
        ///
        /// **Validates: Requirements 3.2, 13.1**
        ///
        /// For any valid `BridgeRateLimit`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_bridge_rate_limit(limit in arb_bridge_rate_limit()) {
            let json = serde_json::to_string(&limit).unwrap();
            let back: BridgeRateLimit = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, limit);
        }
    }
}
