//! Streaming envelope types for WebSocket message framing.
//!
//! This module defines the unified WebSocket message format used by all
//! streaming operations (TTS audio, LLM text, STT transcription, etc.).
//! Only the data types and constructors are included here — stream
//! management and WebSocket dispatch logic stay in the agent crate.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error_chain::ErrorChain;

// ---------------------------------------------------------------------------
// StreamStatus
// ---------------------------------------------------------------------------

/// Status of a streaming envelope chunk.
///
/// - `Data`     — payload carries stream content.
/// - `Complete` — stream finished successfully; no further messages on this `stream_id`.
/// - `Error`    — stream terminated with an error; payload contains a serialized [`ErrorChain`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamStatus {
    Data,
    Complete,
    Error,
}

// ---------------------------------------------------------------------------
// StreamMetadata
// ---------------------------------------------------------------------------

/// Per-stream metadata attached to every [`StreamingEnvelope`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamMetadata {
    /// Which system component produced this stream (e.g. `"tts"`, `"llm"`, `"stt"`).
    pub source_component: String,
    /// Correlation ID linking this stream to the originating request.
    pub correlation_id: String,
    /// Total number of chunks in the stream, if known ahead of time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_chunks: Option<u64>,
    /// Component-specific metadata (e.g. `sample_rate`, `model`).
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}


// ---------------------------------------------------------------------------
// StreamingEnvelope
// ---------------------------------------------------------------------------

/// Unified WebSocket message format for all streaming operations.
///
/// Supports multiplexed streams via `stream_id`, monotonic sequencing via
/// `sequence`, and MIME-typed payloads via `content_type`.
///
/// Error termination: when `status` is [`StreamStatus::Error`], `payload`
/// contains a JSON-serialized [`ErrorChain`]. No further messages will be
/// sent on the same `stream_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamingEnvelope {
    /// Unique identifier for this stream. Multiple concurrent streams are
    /// distinguished by their `stream_id` on the same WebSocket connection.
    pub stream_id: String,
    /// Monotonically increasing sequence number within a stream (starts at 0).
    pub sequence: u64,
    /// MIME content type: `audio/mpeg`, `text/plain`, `image/png`, `application/json`, etc.
    pub content_type: String,
    /// Base64-encoded binary or UTF-8 text payload.
    pub payload: String,
    /// Current status of this chunk.
    pub status: StreamStatus,
    /// Stream metadata including source component and correlation ID.
    pub metadata: StreamMetadata,
}

impl StreamingEnvelope {
    /// Create a new data envelope.
    pub fn data(
        stream_id: impl Into<String>,
        sequence: u64,
        content_type: impl Into<String>,
        payload: impl Into<String>,
        metadata: StreamMetadata,
    ) -> Self {
        Self {
            stream_id: stream_id.into(),
            sequence,
            content_type: content_type.into(),
            payload: payload.into(),
            status: StreamStatus::Data,
            metadata,
        }
    }

    /// Create a completion envelope (no payload).
    pub fn complete(
        stream_id: impl Into<String>,
        sequence: u64,
        content_type: impl Into<String>,
        metadata: StreamMetadata,
    ) -> Self {
        Self {
            stream_id: stream_id.into(),
            sequence,
            content_type: content_type.into(),
            payload: String::new(),
            status: StreamStatus::Complete,
            metadata,
        }
    }

    /// Create an error termination envelope with an [`ErrorChain`] payload.
    pub fn error(
        stream_id: impl Into<String>,
        sequence: u64,
        content_type: impl Into<String>,
        error_chain: &ErrorChain,
        metadata: StreamMetadata,
    ) -> Self {
        let payload = serde_json::to_string(error_chain)
            .unwrap_or_else(|_| r#"{"error":"serialization_failed"}"#.to_string());
        Self {
            stream_id: stream_id.into(),
            sequence,
            content_type: content_type.into(),
            payload,
            status: StreamStatus::Error,
            metadata,
        }
    }

    /// Returns `true` when this envelope terminates the stream (complete or error).
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, StreamStatus::Complete | StreamStatus::Error)
    }

    /// Attempt to deserialize the payload as an [`ErrorChain`].
    ///
    /// Only meaningful when `status` is [`StreamStatus::Error`].
    pub fn error_chain(&self) -> Option<ErrorChain> {
        if self.status == StreamStatus::Error {
            serde_json::from_str(&self.payload).ok()
        } else {
            None
        }
    }
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_chain::{ErrorContext, Subsystem};
    use proptest::prelude::*;

    // -- Helpers -----------------------------------------------------------

    fn sample_metadata() -> StreamMetadata {
        StreamMetadata {
            source_component: "tts".to_string(),
            correlation_id: "corr-123".to_string(),
            total_chunks: Some(10),
            extra: HashMap::new(),
        }
    }

    // -- Unit tests -------------------------------------------------------

    #[test]
    fn data_envelope_round_trip() {
        let env = StreamingEnvelope::data(
            "stream-1",
            0,
            "audio/mpeg",
            "base64data==",
            sample_metadata(),
        );
        let json = serde_json::to_string(&env).unwrap();
        let deserialized: StreamingEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(env, deserialized);
        assert_eq!(deserialized.status, StreamStatus::Data);
        assert!(!deserialized.is_terminal());
    }

    #[test]
    fn complete_envelope_is_terminal() {
        let env = StreamingEnvelope::complete("stream-1", 5, "text/plain", sample_metadata());
        assert!(env.is_terminal());
        assert_eq!(env.status, StreamStatus::Complete);
        assert!(env.payload.is_empty());
    }

    #[test]
    fn error_envelope_contains_error_chain() {
        let mut chain = ErrorChain::new("corr-123".to_string());
        chain.push(ErrorContext::new(
            Subsystem::Provider,
            "PROVIDER_TIMEOUT",
            "Provider timed out after 30s",
        ));

        let env = StreamingEnvelope::error(
            "stream-1",
            3,
            "application/json",
            &chain,
            sample_metadata(),
        );

        assert!(env.is_terminal());
        assert_eq!(env.status, StreamStatus::Error);

        let recovered = env.error_chain().expect("should deserialize ErrorChain");
        assert_eq!(recovered.correlation_id, "corr-123");
        assert_eq!(recovered.contexts.len(), 1);
        assert_eq!(recovered.contexts[0].code, "PROVIDER_TIMEOUT");
    }

    #[test]
    fn error_chain_returns_none_for_data_status() {
        let env = StreamingEnvelope::data("s", 0, "text/plain", "hello", sample_metadata());
        assert!(env.error_chain().is_none());
    }

    #[test]
    fn metadata_extra_fields_flatten() {
        let mut meta = sample_metadata();
        meta.extra
            .insert("sample_rate".to_string(), serde_json::json!(44100));
        meta.extra
            .insert("model".to_string(), serde_json::json!("llama3"));

        let env = StreamingEnvelope::data("s", 0, "audio/mpeg", "data", meta);
        let json = serde_json::to_string(&env).unwrap();

        // Extra fields should appear at the metadata level (flattened)
        let v: Value = serde_json::from_str(&json).unwrap();
        let md = &v["metadata"];
        assert_eq!(md["sample_rate"], 44100);
        assert_eq!(md["model"], "llama3");
    }

    #[test]
    fn stream_status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&StreamStatus::Data).unwrap(),
            "\"data\""
        );
        assert_eq!(
            serde_json::to_string(&StreamStatus::Complete).unwrap(),
            "\"complete\""
        );
        assert_eq!(
            serde_json::to_string(&StreamStatus::Error).unwrap(),
            "\"error\""
        );
    }

    #[test]
    fn multiplexed_streams_distinguished_by_id() {
        let meta = sample_metadata();
        let env_a = StreamingEnvelope::data("stream-a", 0, "text/plain", "hello", meta.clone());
        let env_b = StreamingEnvelope::data("stream-b", 0, "audio/mpeg", "audio", meta);
        assert_ne!(env_a.stream_id, env_b.stream_id);
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

    fn arb_stream_status() -> impl Strategy<Value = StreamStatus> {
        prop_oneof![
            Just(StreamStatus::Data),
            Just(StreamStatus::Complete),
            Just(StreamStatus::Error),
        ]
    }

    fn arb_stream_metadata() -> impl Strategy<Value = StreamMetadata> {
        (
            "\\w{1,20}",
            "\\w{1,30}",
            proptest::option::of(any::<u64>()),
            proptest::collection::hash_map("\\w{1,10}", arb_json_value(), 0..3),
        )
            .prop_map(
                |(source_component, correlation_id, total_chunks, extra)| StreamMetadata {
                    source_component,
                    correlation_id,
                    total_chunks,
                    extra,
                },
            )
    }

    fn arb_streaming_envelope() -> impl Strategy<Value = StreamingEnvelope> {
        (
            "\\w{1,20}",
            any::<u64>(),
            "\\w{1,20}",
            "\\w{0,50}",
            arb_stream_status(),
            arb_stream_metadata(),
        )
            .prop_map(
                |(stream_id, sequence, content_type, payload, status, metadata)| {
                    StreamingEnvelope {
                        stream_id,
                        sequence,
                        content_type,
                        payload,
                        status,
                        metadata,
                    }
                },
            )
    }

    // -- Property tests ---------------------------------------------------

    proptest! {
        /// Property 1: Serde JSON round-trip for streaming envelope types
        ///
        /// **Validates: Requirements 4.2, 13.1**
        ///
        /// For any valid `StreamStatus`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_stream_status(status in arb_stream_status()) {
            let json = serde_json::to_string(&status).unwrap();
            let back: StreamStatus = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, status);
        }

        /// Property 1: Serde JSON round-trip for streaming envelope types
        ///
        /// **Validates: Requirements 4.2, 13.1**
        ///
        /// For any valid `StreamMetadata`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_stream_metadata(meta in arb_stream_metadata()) {
            let json = serde_json::to_string(&meta).unwrap();
            let back: StreamMetadata = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, meta);
        }

        /// Property 1: Serde JSON round-trip for streaming envelope types
        ///
        /// **Validates: Requirements 4.2, 13.1**
        ///
        /// For any valid `StreamingEnvelope`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_streaming_envelope(env in arb_streaming_envelope()) {
            let json = serde_json::to_string(&env).unwrap();
            let back: StreamingEnvelope = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, env);
        }
    }
}
