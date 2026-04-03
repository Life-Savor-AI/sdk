//! Minimal TTS system component example.
//!
//! Demonstrates building a text-to-speech system component using the
//! `SystemComponentBuilder`, producing audio output via `StreamingEnvelope`,
//! and implementing a health check.
//!
//! Run with: `cargo run --example tts_component`

use std::collections::HashMap;

use lifesavor_system_sdk::prelude::*;
use lifesavor_system_sdk::builder::SystemComponentBuilder;
use lifesavor_system_sdk::health::HealthCheckBuilder;
use lifesavor_system_sdk::testing::MockAgentContext;
use lifesavor_system_sdk::{HealthCheckConfig, HealthCheckMethod};

/// Synthesize text into a sequence of `StreamingEnvelope` audio chunks.
///
/// In a real component this would call an audio engine; here we produce
/// a single base64-encoded placeholder chunk followed by a completion
/// envelope.
#[instrument(skip_all, fields(text_len = text.len()))]
fn synthesize(text: &str, correlation_id: &str) -> Vec<StreamingEnvelope> {
    let stream_id = uuid::Uuid::new_v4().to_string();
    let metadata = StreamMetadata {
        source_component: "tts".to_string(),
        correlation_id: correlation_id.to_string(),
        total_chunks: Some(2),
        extra: HashMap::new(),
    };

    let data_chunk = StreamingEnvelope::data(
        &stream_id,
        0,
        "audio/mpeg",
        // Placeholder payload — real component would produce actual audio bytes.
        base64_placeholder(text),
        metadata.clone(),
    );

    let complete = StreamingEnvelope::complete(&stream_id, 1, "audio/mpeg", metadata);

    info!(stream_id = %stream_id, chunks = 2, "TTS synthesis complete");
    vec![data_chunk, complete]
}

/// Produce a deterministic placeholder "audio" payload from the input text.
fn base64_placeholder(text: &str) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    let _ = write!(buf, "AUDIO:{}", text);
    base64_encode(&buf)
}

/// Minimal base64 encoder (avoids pulling in the `base64` crate for an example).
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[tokio::main]
#[instrument]
async fn main() {
    // Initialize structured tracing.
    tracing_subscriber::fmt::init();

    info!("Building TTS system component");

    // Build the component using the SDK builder.
    // Note: SystemComponentType::Tts requires the `tts` feature flag.
    // We use Cache here for portability; in a real TTS component you would
    // enable the `tts` feature and use SystemComponentType::Tts.
    let component = SystemComponentBuilder::new("tts", SystemComponentType::Cache)
        .on_initialize(|| {
            Box::pin(async {
                info!("TTS component initialized");
                Ok(())
            })
        })
        .on_health_check(|| {
            Box::pin(async {
                // A real component would verify its audio engine is ready.
                ComponentHealthStatus::Healthy
            })
        })
        .on_shutdown(|| {
            Box::pin(async {
                info!("TTS component shutting down");
                Ok(())
            })
        })
        .build()
        .expect("TTS component builder should succeed");

    // Run the component through the mock lifecycle.
    let mut ctx = MockAgentContext::new();
    ctx.register(component);
    ctx.initialize().await.expect("initialize should succeed");

    let health = ctx.health_check().await.expect("health check should succeed");
    info!(?health, "Health check result");

    // Configure a health check builder from manifest config.
    let hc_builder = HealthCheckBuilder::new(HealthCheckConfig {
        interval_seconds: 30,
        timeout_seconds: 5,
        consecutive_failures_threshold: 3,
        method: HealthCheckMethod::ConnectionPing,
    });
    let hc_status = hc_builder.check().await;
    info!(?hc_status, "HealthCheckBuilder probe result");

    // Demonstrate TTS synthesis producing StreamingEnvelope output.
    let envelopes = synthesize("Hello from the TTS component!", "corr-001");
    for env in &envelopes {
        info!(
            stream_id = %env.stream_id,
            seq = env.sequence,
            status = ?env.status,
            content_type = %env.content_type,
            "Streaming envelope"
        );
    }

    ctx.shutdown().await.expect("shutdown should succeed");
    info!("TTS example complete");
}
