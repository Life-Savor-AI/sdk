//! Error chain types for cross-subsystem error reporting.
//!
//! This module defines the structured error chain types used to accumulate
//! error contexts as a request traverses agent subsystems. Only the data
//! types are included here — `AgentError` and agent-specific error handling
//! stay in the agent crate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Subsystem
// ---------------------------------------------------------------------------

/// Identifies which agent subsystem produced an error context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subsystem {
    Router,
    Provider,
    Interceptor,
    Sandbox,
    EventStore,
    MetricsStore,
    Vault,
    Scheduler,
    Registry,
    Identity,
    ContentSafety,
    Bridge,
    ProcessManager,
    /// LLM inference subsystem (Ollama component).
    Llm,
}

// ---------------------------------------------------------------------------
// ErrorContext
// ---------------------------------------------------------------------------

/// A single error context produced by a subsystem.
///
/// Contexts can be nested via `children` to represent multi-hop failures
/// (e.g. router → provider A failed → failover → provider B failed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Which subsystem produced this context.
    pub subsystem: Subsystem,
    /// Machine-readable error code (e.g. `"PROVIDER_TIMEOUT"`, `"PII_BLOCKED"`).
    pub code: String,
    /// Human-readable, locale-aware message.
    pub message: String,
    /// Optional i18n template key for localized rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale_key: Option<String>,
    /// Subsystem-specific metadata (free-form JSON).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// Nested child contexts for multi-hop / cascading failures.
    #[serde(default)]
    pub children: Vec<ErrorContext>,
}

impl ErrorContext {
    /// Convenience constructor for a leaf error context (no children).
    pub fn new(subsystem: Subsystem, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            subsystem,
            code: code.into(),
            message: message.into(),
            locale_key: None,
            metadata: None,
            children: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// ErrorChain
// ---------------------------------------------------------------------------

/// An ordered chain of [`ErrorContext`]s accumulated as a request traverses
/// the agent's subsystems. Included in `ExecutionFailed` messages so the
/// web-app and LLM can present a step-by-step failure explanation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorChain {
    /// Correlation ID linking this chain to the originating request.
    pub correlation_id: String,
    /// Timestamp when the chain was finalized.
    pub timestamp: DateTime<Utc>,
    /// Ordered list of error contexts, one per subsystem hop.
    pub contexts: Vec<ErrorContext>,
}

impl ErrorChain {
    /// Create a new, empty error chain for the given correlation ID.
    pub fn new(correlation_id: String) -> Self {
        Self {
            correlation_id,
            timestamp: Utc::now(),
            contexts: Vec::new(),
        }
    }

    /// Append an error context to the chain.
    pub fn push(&mut self, ctx: ErrorContext) {
        self.contexts.push(ctx);
    }

    /// Returns `true` when the chain contains no error contexts.
    pub fn is_empty(&self) -> bool {
        self.contexts.is_empty()
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
    fn error_chain_new_is_empty() {
        let chain = ErrorChain::new("corr-1".to_string());
        assert!(chain.is_empty());
        assert_eq!(chain.correlation_id, "corr-1");
        assert!(chain.contexts.is_empty());
    }

    #[test]
    fn error_chain_push_and_is_empty() {
        let mut chain = ErrorChain::new("corr-2".to_string());
        assert!(chain.is_empty());
        chain.push(ErrorContext::new(
            Subsystem::Router,
            "ROUTE_FAIL",
            "no route found",
        ));
        assert!(!chain.is_empty());
        assert_eq!(chain.contexts.len(), 1);
    }

    #[test]
    fn error_context_new_defaults() {
        let ctx = ErrorContext::new(Subsystem::Provider, "TIMEOUT", "timed out");
        assert_eq!(ctx.subsystem, Subsystem::Provider);
        assert_eq!(ctx.code, "TIMEOUT");
        assert_eq!(ctx.message, "timed out");
        assert!(ctx.locale_key.is_none());
        assert!(ctx.metadata.is_none());
        assert!(ctx.children.is_empty());
    }

    #[test]
    fn subsystem_serde_unit() {
        let json = serde_json::to_string(&Subsystem::ContentSafety).unwrap();
        assert_eq!(json, "\"ContentSafety\"");
        let back: Subsystem = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Subsystem::ContentSafety);
    }

    #[test]
    fn error_context_with_children_round_trip() {
        let child = ErrorContext::new(Subsystem::Provider, "PROVIDER_B_FAIL", "provider B down");
        let mut parent = ErrorContext::new(Subsystem::Router, "FAILOVER", "failover triggered");
        parent.children.push(child);

        let json = serde_json::to_string(&parent).unwrap();
        let back: ErrorContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back, parent);
        assert_eq!(back.children.len(), 1);
    }

    #[test]
    fn error_chain_serde_unit() {
        let mut chain = ErrorChain::new("corr-100".to_string());
        chain.push(ErrorContext::new(
            Subsystem::Vault,
            "VAULT_LOCKED",
            "vault is sealed",
        ));
        let json = serde_json::to_string(&chain).unwrap();
        let back: ErrorChain = serde_json::from_str(&json).unwrap();
        assert_eq!(back, chain);
    }

    #[test]
    fn error_context_skip_serializing_none_fields() {
        let ctx = ErrorContext::new(Subsystem::Bridge, "ERR", "msg");
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(!json.contains("locale_key"));
        assert!(!json.contains("metadata"));
    }

    // -- Proptest strategies ----------------------------------------------

    fn arb_subsystem() -> impl Strategy<Value = Subsystem> {
        prop_oneof![
            Just(Subsystem::Router),
            Just(Subsystem::Provider),
            Just(Subsystem::Interceptor),
            Just(Subsystem::Sandbox),
            Just(Subsystem::EventStore),
            Just(Subsystem::MetricsStore),
            Just(Subsystem::Vault),
            Just(Subsystem::Scheduler),
            Just(Subsystem::Registry),
            Just(Subsystem::Identity),
            Just(Subsystem::ContentSafety),
            Just(Subsystem::Bridge),
            Just(Subsystem::ProcessManager),
            Just(Subsystem::Llm),
        ]
    }

    /// Non-null JSON values only — `Value::Null` inside `Some(…)` is lossy
    /// through serde `Option` round-trips (`Some(Null)` → JSON `null` → `None`).
    fn arb_json_value_non_null() -> impl Strategy<Value = Value> {
        prop_oneof![
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(|n| Value::Number(n.into())),
            "\\w{0,20}".prop_map(|s| Value::String(s)),
        ]
    }

    fn arb_error_context_leaf() -> impl Strategy<Value = ErrorContext> {
        (
            arb_subsystem(),
            "\\w{1,20}",
            "\\w{1,50}",
            proptest::option::of("\\w{1,30}"),
            proptest::option::of(arb_json_value_non_null()),
        )
            .prop_map(|(subsystem, code, message, locale_key, metadata)| {
                ErrorContext {
                    subsystem,
                    code,
                    message,
                    locale_key,
                    metadata,
                    children: Vec::new(),
                }
            })
    }

    /// Strategy that generates an `ErrorContext` with up to 3 leaf children.
    fn arb_error_context() -> impl Strategy<Value = ErrorContext> {
        (
            arb_error_context_leaf(),
            proptest::collection::vec(arb_error_context_leaf(), 0..3),
        )
            .prop_map(|(mut ctx, children)| {
                ctx.children = children;
                ctx
            })
    }

    fn arb_datetime_utc() -> impl Strategy<Value = DateTime<Utc>> {
        // Generate timestamps in a reasonable range (2020-01-01 to 2030-01-01)
        (1577836800i64..1893456000i64).prop_map(|secs| {
            DateTime::from_timestamp(secs, 0).unwrap()
        })
    }

    fn arb_error_chain() -> impl Strategy<Value = ErrorChain> {
        (
            "\\w{1,30}",
            arb_datetime_utc(),
            proptest::collection::vec(arb_error_context(), 0..5),
        )
            .prop_map(|(correlation_id, timestamp, contexts)| {
                ErrorChain {
                    correlation_id,
                    timestamp,
                    contexts,
                }
            })
    }

    // -- Property tests ---------------------------------------------------

    proptest! {
        /// Property 1: Serde JSON round-trip for error chain types
        ///
        /// **Validates: Requirements 5.1, 13.1**
        ///
        /// For any valid `Subsystem`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_subsystem(sub in arb_subsystem()) {
            let json = serde_json::to_string(&sub).unwrap();
            let back: Subsystem = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, sub);
        }

        /// Property 1: Serde JSON round-trip for error chain types
        ///
        /// **Validates: Requirements 5.1, 13.1**
        ///
        /// For any valid `ErrorContext` (with nested children),
        /// serializing to JSON and deserializing back produces the
        /// original value.
        #[test]
        fn serde_round_trip_error_context(ctx in arb_error_context()) {
            let json = serde_json::to_string(&ctx).unwrap();
            let back: ErrorContext = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, ctx);
        }

        /// Property 1: Serde JSON round-trip for error chain types
        ///
        /// **Validates: Requirements 5.1, 13.1**
        ///
        /// For any valid `ErrorChain`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_error_chain(chain in arb_error_chain()) {
            let json = serde_json::to_string(&chain).unwrap();
            let back: ErrorChain = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, chain);
        }
    }
}
