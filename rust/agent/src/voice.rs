//! Voice types and traits for the Life Savor agent SDK.
//!
//! This module provides types and traits for voice transcription, synthesis,
//! and pipeline orchestration, including:
//!
//! - [`VoiceConfig`] — Configuration for voice components
//! - [`VoiceTranscriber`] — Trait for speech-to-text (STT) components
//! - [`VoiceSynthesizer`] — Trait for text-to-speech (TTS) components
//! - [`PipelineSessionMetrics`] — Metrics from a complete voice pipeline session
//! - [`TranscriptionMetrics`] — Per-request STT metrics
//! - [`TranscriptionRequest`] — Request to transcribe audio
//! - [`TranscriptionResult`] — Result of batch transcription
//! - [`SynthesisMetrics`] — Per-request TTS metrics
//! - [`SynthesisRequest`] — Request to synthesize text to audio
//! - [`VoiceInfo`] — Metadata describing a TTS voice

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::streaming::StreamingEnvelope;

// ---------------------------------------------------------------------------
// VoiceConfig
// ---------------------------------------------------------------------------

/// Configuration for voice components.
///
/// Includes settings for language preferences, audio format defaults,
/// and PII interception behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// Default language hint (BCP-47 tag, e.g., "en-US").
    pub default_language: String,
    /// Default audio format MIME type (e.g., "audio/wav", "audio/mpeg").
    pub default_audio_format: String,
    /// Whether PII interception is enabled in the voice pipeline.
    pub pii_interception_enabled: bool,
}

// ---------------------------------------------------------------------------
// Transcription Types
// ---------------------------------------------------------------------------

/// Request to transcribe audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRequest {
    /// Correlation ID for distributed tracing.
    pub correlation_id: String,
    /// Language hint (BCP-47 tag). If `None`, automatic detection is attempted.
    pub language_hint: Option<String>,
    /// Audio content type MIME (e.g., "audio/wav", "audio/webm").
    pub audio_format: String,
    /// Sample rate in Hz, if known (e.g., 16000).
    pub sample_rate: Option<u32>,
    /// Optional assistant ID for language preference lookup.
    pub assistant_id: Option<String>,
    /// Optional user ID for preference-based language selection.
    pub user_id: Option<String>,
}

/// Per-request metrics recorded after transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionMetrics {
    /// Wall-clock transcription latency in milliseconds.
    pub latency_ms: u64,
    /// Duration of the input audio in milliseconds.
    pub audio_duration_ms: u64,
    /// Number of words in the final transcript.
    pub word_count: usize,
    /// Overall confidence score in [0.0, 1.0].
    pub confidence: Option<f64>,
    /// Provider that performed the transcription.
    pub provider: String,
    /// Detected or used language.
    pub language: Option<String>,
    /// Timestamp of the transcription.
    pub timestamp: DateTime<Utc>,
    /// Correlation ID.
    pub correlation_id: String,
}

/// Result of a batch (non-streaming) transcription call.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// The complete transcribed text.
    pub text: String,
    /// Per-request metrics.
    pub metrics: TranscriptionMetrics,
}

// ---------------------------------------------------------------------------
// Synthesis Types
// ---------------------------------------------------------------------------

/// Request to synthesize text into audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisRequest {
    /// Text to synthesize. PII tokens will be replaced with spoken placeholders.
    pub text: String,
    /// Voice to use. Falls back to assistant default → user preference → provider default.
    pub voice_id: Option<String>,
    /// Desired audio output format MIME type (e.g., "audio/mpeg", "audio/wav").
    pub output_format: Option<String>,
    /// Correlation ID for tracing.
    pub correlation_id: String,
    /// Optional assistant ID for voice selection fallback.
    pub assistant_id: Option<String>,
    /// Optional user ID for preference-based voice selection.
    pub user_id: Option<String>,
}

/// Per-request metrics recorded after synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisMetrics {
    /// Wall-clock synthesis latency in milliseconds.
    pub latency_ms: u64,
    /// Duration of the generated audio in milliseconds.
    pub audio_duration_ms: u64,
    /// Number of characters synthesized.
    pub character_count: usize,
    /// Provider that performed the synthesis.
    pub provider: String,
    /// Voice used.
    pub voice_id: String,
    /// Timestamp of the synthesis.
    pub timestamp: DateTime<Utc>,
    /// Correlation ID.
    pub correlation_id: String,
}

// ---------------------------------------------------------------------------
// Voice Metadata
// ---------------------------------------------------------------------------

/// Gender hint for a TTS voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceGender {
    /// Male voice.
    Male,
    /// Female voice.
    Female,
    /// Neutral/non-binary voice.
    Neutral,
}

/// Metadata describing a single TTS voice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceInfo {
    /// Unique voice identifier within the provider (e.g., "en_US-amy-medium").
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// BCP-47 language tag (e.g., "en-US", "es-MX").
    pub language: String,
    /// Voice gender hint.
    pub gender: VoiceGender,
    /// Whether this is the provider's default voice.
    pub is_default: bool,
    /// Provider-specific extra metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Pipeline Metrics
// ---------------------------------------------------------------------------

/// Metrics from a complete voice pipeline session (STT → LLM → TTS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSessionMetrics {
    /// Total end-to-end latency in milliseconds.
    pub total_latency_ms: u64,
    /// STT stage latency in milliseconds.
    pub stt_latency_ms: u64,
    /// LLM inference latency in milliseconds.
    pub llm_latency_ms: u64,
    /// TTS stage latency in milliseconds.
    pub tts_latency_ms: u64,
    /// Number of words in the STT transcript.
    pub stt_word_count: usize,
    /// Number of characters synthesized by TTS.
    pub tts_character_count: usize,
    /// Correlation ID for distributed tracing.
    pub correlation_id: String,
    /// Timestamp when the session completed.
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// VoiceTranscriber Trait
// ---------------------------------------------------------------------------

/// Trait for speech-to-text (STT) components.
///
/// Implementations provide audio transcription, both streaming and batch.
/// Streaming results are sent back via the Streaming Envelope protocol.
#[async_trait]
pub trait VoiceTranscriber: Send + Sync {
    /// Transcribe a stream of audio chunks.
    ///
    /// Audio chunks arrive via `rx` as [`StreamingEnvelope`] messages.
    /// Partial and final transcript segments are sent through `tx`.
    ///
    /// # Arguments
    ///
    /// - `request` — Transcription request with language hints and metadata
    /// - `rx` — Receiver for audio chunks
    /// - `tx` — Sender for transcript envelopes
    ///
    /// # Returns
    ///
    /// [`TranscriptionMetrics`] on success, or an error if transcription fails.
    async fn transcribe_stream(
        &self,
        request: &TranscriptionRequest,
        rx: &mut mpsc::Receiver<StreamingEnvelope>,
        tx: mpsc::Sender<StreamingEnvelope>,
    ) -> Result<TranscriptionMetrics, Box<dyn std::error::Error + Send + Sync>>;

    /// Transcribe a complete audio buffer in one shot.
    ///
    /// # Arguments
    ///
    /// - `request` — Transcription request with language hints and metadata
    /// - `audio_data` — Complete audio buffer
    ///
    /// # Returns
    ///
    /// [`TranscriptionResult`] containing text and metrics on success,
    /// or an error if transcription fails.
    async fn transcribe_batch(
        &self,
        request: &TranscriptionRequest,
        audio_data: &[u8],
    ) -> Result<TranscriptionResult, Box<dyn std::error::Error + Send + Sync>>;
}

// ---------------------------------------------------------------------------
// VoiceSynthesizer Trait
// ---------------------------------------------------------------------------

/// Trait for text-to-speech (TTS) components.
///
/// Implementations provide audio synthesis and voice enumeration.
/// Audio is streamed back to clients via the Streaming Envelope protocol.
#[async_trait]
pub trait VoiceSynthesizer: Send + Sync {
    /// Synthesize text into audio and stream chunks.
    ///
    /// Synthesized audio chunks are sent through `tx` as [`StreamingEnvelope`]
    /// messages with the negotiated audio MIME type.
    ///
    /// # Arguments
    ///
    /// - `request` — Synthesis request with text and voice preferences
    /// - `tx` — Sender for audio envelopes
    ///
    /// # Returns
    ///
    /// [`SynthesisMetrics`] on success, or an error if synthesis fails.
    async fn synthesize(
        &self,
        request: &SynthesisRequest,
        tx: mpsc::Sender<StreamingEnvelope>,
    ) -> Result<SynthesisMetrics, Box<dyn std::error::Error + Send + Sync>>;

    /// List available voices with language and gender metadata.
    ///
    /// # Returns
    ///
    /// A vector of [`VoiceInfo`] describing available voices,
    /// or an error if the operation fails.
    async fn list_voices(
        &self,
    ) -> Result<Vec<VoiceInfo>, Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_config_serde_round_trip() {
        let config = VoiceConfig {
            default_language: "en-US".to_string(),
            default_audio_format: "audio/wav".to_string(),
            pii_interception_enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: VoiceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.default_language, "en-US");
        assert_eq!(restored.default_audio_format, "audio/wav");
        assert!(restored.pii_interception_enabled);
    }

    #[test]
    fn transcription_request_serde_round_trip() {
        let req = TranscriptionRequest {
            correlation_id: "corr-1".to_string(),
            language_hint: Some("en-US".to_string()),
            audio_format: "audio/wav".to_string(),
            sample_rate: Some(16000),
            assistant_id: None,
            user_id: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let restored: TranscriptionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.correlation_id, "corr-1");
        assert_eq!(restored.language_hint, Some("en-US".to_string()));
    }

    #[test]
    fn voice_info_serde_round_trip() {
        let voice = VoiceInfo {
            id: "en_US-amy-medium".to_string(),
            name: "Amy".to_string(),
            language: "en-US".to_string(),
            gender: VoiceGender::Female,
            is_default: true,
            extra: HashMap::new(),
        };
        let json = serde_json::to_string(&voice).unwrap();
        let restored: VoiceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "en_US-amy-medium");
        assert_eq!(restored.gender, VoiceGender::Female);
    }

    #[test]
    fn pipeline_metrics_serde_round_trip() {
        let metrics = PipelineSessionMetrics {
            total_latency_ms: 3500,
            stt_latency_ms: 1250,
            llm_latency_ms: 400,
            tts_latency_ms: 850,
            stt_word_count: 42,
            tts_character_count: 156,
            correlation_id: "corr-1".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&metrics).unwrap();
        let restored: PipelineSessionMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_latency_ms, 3500);
        assert_eq!(restored.stt_word_count, 42);
    }
}
