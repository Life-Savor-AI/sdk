//! Analytics event reporting for the Developer Portal (Req 33).
//!
//! Gated behind the `analytics` feature flag. The [`AnalyticsReporter`]
//! batches events and flushes them asynchronously to the Developer Portal
//! API, with retry and local buffering on failure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Event types supported by the Developer Portal analytics API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventType {
    Install,
    Uninstall,
    DailyUsagePing,
    PageView,
}

/// A single analytics event destined for the Developer Portal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub event_type: AnalyticsEventType,
    pub component_id: String,
    pub version: String,
    pub platform: String,
    pub timestamp: DateTime<Utc>,
    pub payload: Option<serde_json::Value>,
}

/// Errors that can occur during analytics reporting.
#[derive(Debug, thiserror::Error)]
pub enum AnalyticsError {
    #[error("flush failed after {retries} retries: {message}")]
    FlushFailed { retries: u32, message: String },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Reporter
// ---------------------------------------------------------------------------

/// Default endpoint for the Developer Portal analytics API.
const DEFAULT_ENDPOINT: &str = "https://developer.stage.lifesavor.ai/api/v3/developer/analytics/events";

/// Maximum number of failed events to buffer locally.
const MAX_FAILED_BUFFER: usize = 500;

/// Maximum retry attempts per flush.
const MAX_RETRIES: u32 = 3;

/// Batches analytics events and flushes them to the Developer Portal API.
///
/// # Example
///
/// ```rust,ignore
/// use lifesavor_system_sdk::analytics::{AnalyticsReporter, AnalyticsEvent, AnalyticsEventType};
///
/// let mut reporter = AnalyticsReporter::new("ls_dev_my_key");
/// reporter.report(AnalyticsEvent {
///     event_type: AnalyticsEventType::DailyUsagePing,
///     component_id: "my-component".into(),
///     version: "0.1.0".into(),
///     platform: "linux-x86_64".into(),
///     timestamp: chrono::Utc::now(),
///     payload: None,
/// });
/// // reporter.flush().await?;
/// ```
pub struct AnalyticsReporter {
    api_key: String,
    endpoint: String,
    batch: Vec<AnalyticsEvent>,
    flush_interval: Duration,
    max_batch_size: usize,
    failed_buffer: Vec<AnalyticsEvent>,
}

impl AnalyticsReporter {
    /// Create a new reporter with the given Developer Portal API key.
    ///
    /// Uses the default endpoint and batch settings (flush every 60 s or
    /// 50 events, whichever comes first).
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_owned(),
            endpoint: DEFAULT_ENDPOINT.to_owned(),
            batch: Vec::new(),
            flush_interval: Duration::from_secs(60),
            max_batch_size: 50,
            failed_buffer: Vec::new(),
        }
    }

    /// Override the default endpoint URL.
    pub fn with_endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = endpoint.to_owned();
        self
    }

    /// Override the default flush interval.
    pub fn with_flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = interval;
        self
    }

    /// Override the default max batch size.
    pub fn with_max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    /// Return the configured flush interval.
    pub fn flush_interval(&self) -> Duration {
        self.flush_interval
    }

    /// Return the configured max batch size.
    pub fn max_batch_size(&self) -> usize {
        self.max_batch_size
    }

    /// Return the number of events currently in the pending batch.
    pub fn pending_count(&self) -> usize {
        self.batch.len()
    }

    /// Return the number of events in the failed-event buffer.
    pub fn failed_count(&self) -> usize {
        self.failed_buffer.len()
    }

    /// Add an event to the current batch.
    ///
    /// Does **not** trigger a flush — call [`flush`](Self::flush) explicitly
    /// or rely on a background timer.
    pub fn report(&mut self, event: AnalyticsEvent) {
        self.batch.push(event);
    }

    /// Flush all pending events (including any previously failed events) to
    /// the Developer Portal API.
    ///
    /// Uses a mock/no-op HTTP client for now. In production this would POST
    /// the serialised batch to `self.endpoint` with the API key in an
    /// `Authorization` header.
    ///
    /// On failure the events are moved to the failed buffer (capped at 500)
    /// and a `tracing::warn!` is emitted.
    pub async fn flush(&mut self) -> Result<(), AnalyticsError> {
        if self.batch.is_empty() && self.failed_buffer.is_empty() {
            return Ok(());
        }

        // Merge previously failed events into the current batch.
        let mut events_to_send: Vec<AnalyticsEvent> =
            self.failed_buffer.drain(..).chain(self.batch.drain(..)).collect();

        // Attempt to send with exponential backoff.
        let mut last_err: Option<String> = None;
        for attempt in 0..MAX_RETRIES {
            match Self::try_send(&self.endpoint, &self.api_key, &events_to_send).await {
                Ok(()) => return Ok(()),
                Err(msg) => {
                    last_err = Some(msg);
                    // Exponential backoff: 100ms, 200ms, 400ms
                    let backoff = Duration::from_millis(100 * 2u64.pow(attempt));
                    tokio::time::sleep(backoff).await;
                }
            }
        }

        // All retries exhausted — buffer the events and warn.
        let err_msg = last_err.unwrap_or_else(|| "unknown error".into());
        tracing::warn!(
            retries = MAX_RETRIES,
            buffered = events_to_send.len(),
            "analytics flush failed after {} retries: {}",
            MAX_RETRIES,
            err_msg,
        );

        // Keep the failed buffer capped at MAX_FAILED_BUFFER.
        let space = MAX_FAILED_BUFFER.saturating_sub(self.failed_buffer.len());
        let to_keep = events_to_send.len().min(space);
        self.failed_buffer
            .extend(events_to_send.drain(..to_keep));

        Err(AnalyticsError::FlushFailed {
            retries: MAX_RETRIES,
            message: err_msg,
        })
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Attempt to POST the batch to the endpoint.
    ///
    /// This is a **mock / no-op** implementation. A real implementation
    /// would use `reqwest` or a similar HTTP client. The mock always
    /// succeeds so that the SDK compiles and tests pass without network
    /// access.
    async fn try_send(
        _endpoint: &str,
        _api_key: &str,
        _events: &[AnalyticsEvent],
    ) -> std::result::Result<(), String> {
        // No-op: always succeeds. Replace with real HTTP POST in production.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event(event_type: AnalyticsEventType) -> AnalyticsEvent {
        AnalyticsEvent {
            event_type,
            component_id: "test-component".into(),
            version: "0.1.0".into(),
            platform: "linux-x86_64".into(),
            timestamp: Utc::now(),
            payload: None,
        }
    }

    #[test]
    fn new_reporter_has_defaults() {
        let r = AnalyticsReporter::new("ls_dev_test");
        assert_eq!(r.flush_interval(), Duration::from_secs(60));
        assert_eq!(r.max_batch_size(), 50);
        assert_eq!(r.pending_count(), 0);
        assert_eq!(r.failed_count(), 0);
    }

    #[test]
    fn report_adds_to_batch() {
        let mut r = AnalyticsReporter::new("ls_dev_test");
        r.report(sample_event(AnalyticsEventType::Install));
        r.report(sample_event(AnalyticsEventType::Uninstall));
        assert_eq!(r.pending_count(), 2);
    }

    #[tokio::test]
    async fn flush_clears_batch() {
        let mut r = AnalyticsReporter::new("ls_dev_test");
        r.report(sample_event(AnalyticsEventType::DailyUsagePing));
        r.report(sample_event(AnalyticsEventType::PageView));
        r.flush().await.unwrap();
        assert_eq!(r.pending_count(), 0);
        assert_eq!(r.failed_count(), 0);
    }

    #[tokio::test]
    async fn flush_empty_is_ok() {
        let mut r = AnalyticsReporter::new("ls_dev_test");
        r.flush().await.unwrap();
    }

    #[test]
    fn builder_methods_work() {
        let r = AnalyticsReporter::new("ls_dev_test")
            .with_endpoint("https://custom.example.com/events")
            .with_flush_interval(Duration::from_secs(30))
            .with_max_batch_size(200);
        assert_eq!(r.flush_interval(), Duration::from_secs(30));
        assert_eq!(r.max_batch_size(), 200);
    }

    #[test]
    fn event_type_serialization() {
        let json = serde_json::to_string(&AnalyticsEventType::DailyUsagePing).unwrap();
        assert_eq!(json, "\"daily_usage_ping\"");
        let rt: AnalyticsEventType = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, AnalyticsEventType::DailyUsagePing);
    }

    #[test]
    fn event_round_trip() {
        let evt = sample_event(AnalyticsEventType::Install);
        let json = serde_json::to_string(&evt).unwrap();
        let rt: AnalyticsEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.event_type, evt.event_type);
        assert_eq!(rt.component_id, evt.component_id);
    }
}
