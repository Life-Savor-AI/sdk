//! Health check builder for the System SDK.
//!
//! Provides [`HealthCheckBuilder`] which constructs a health check
//! implementation from a [`HealthCheckConfig`] in the provider manifest.
//! The builder supports all three health check methods defined in the agent
//! crate: [`HealthCheckMethod::HttpGet`], [`HealthCheckMethod::ConnectionPing`],
//! and [`HealthCheckMethod::CapabilityProbe`].
//!
//! Health checks enforce the configured timeout — if the probe exceeds
//! `timeout_seconds`, a failure status is returned rather than blocking.

use std::time::Duration;

use crate::{ComponentHealthStatus, HealthCheckConfig, HealthCheckMethod};

/// Builder that constructs a runnable health check from a [`HealthCheckConfig`].
///
/// # Example
///
/// ```rust,ignore
/// use lifesavor_system_sdk::health::HealthCheckBuilder;
/// use lifesavor_system_sdk::{HealthCheckConfig, HealthCheckMethod};
///
/// let config = HealthCheckConfig {
///     interval_seconds: 30,
///     timeout_seconds: 5,
///     consecutive_failures_threshold: 3,
///     method: HealthCheckMethod::ConnectionPing,
/// };
///
/// let checker = HealthCheckBuilder::new(config);
/// let status = checker.check().await;
/// ```
pub struct HealthCheckBuilder {
    config: HealthCheckConfig,
}

impl HealthCheckBuilder {
    /// Create a new health check builder from the given config.
    pub fn new(config: HealthCheckConfig) -> Self {
        Self { config }
    }

    /// Return the configured check interval as a [`Duration`].
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.config.interval_seconds)
    }

    /// Return the configured timeout as a [`Duration`].
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.config.timeout_seconds)
    }

    /// Return the consecutive failure threshold.
    pub fn failure_threshold(&self) -> u32 {
        self.config.consecutive_failures_threshold
    }

    /// Return a reference to the underlying config.
    pub fn config(&self) -> &HealthCheckConfig {
        &self.config
    }

    /// Execute the health check, enforcing the configured timeout.
    ///
    /// If the probe exceeds `timeout_seconds`, returns
    /// [`ComponentHealthStatus::Unhealthy`] rather than blocking.
    pub async fn check(&self) -> ComponentHealthStatus {
        let timeout_dur = self.timeout();

        match tokio::time::timeout(timeout_dur, self.run_probe()).await {
            Ok(status) => status,
            Err(_elapsed) => ComponentHealthStatus::Unhealthy {
                details: format!(
                    "Health check timed out after {}s",
                    self.config.timeout_seconds
                ),
            },
        }
    }

    /// Run the actual probe based on the configured method.
    async fn run_probe(&self) -> ComponentHealthStatus {
        match &self.config.method {
            HealthCheckMethod::HttpGet { url } => self.probe_http(url).await,
            HealthCheckMethod::ConnectionPing => self.probe_connection_ping().await,
            HealthCheckMethod::CapabilityProbe => self.probe_capability().await,
        }
    }

    /// HTTP GET probe — attempts a TCP connection to the URL's host and port.
    async fn probe_http(&self, url: &str) -> ComponentHealthStatus {
        match parse_host_port(url) {
            Some(addr) => match tokio::net::TcpStream::connect(&addr).await {
                Ok(_) => ComponentHealthStatus::Healthy,
                Err(e) => ComponentHealthStatus::Unhealthy {
                    details: format!("HTTP health check failed: {e}"),
                },
            },
            None => ComponentHealthStatus::Unhealthy {
                details: format!("Invalid health check URL: {url}"),
            },
        }
    }

    /// Connection ping probe — a lightweight connectivity check.
    ///
    /// For system components this is a no-op success since the component
    /// runs in-process. Real connectivity checks are component-specific.
    async fn probe_connection_ping(&self) -> ComponentHealthStatus {
        ComponentHealthStatus::Healthy
    }

    /// Capability probe — verifies the component can serve its declared
    /// capabilities.
    ///
    /// Default implementation returns Healthy; component developers can
    /// override behavior by wrapping the builder.
    async fn probe_capability(&self) -> ComponentHealthStatus {
        ComponentHealthStatus::Healthy
    }
}

/// Parse a URL string into a `host:port` address for TCP connection.
fn parse_host_port(url: &str) -> Option<String> {
    // Strip scheme
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Split off path
    let authority = without_scheme.split('/').next()?;

    if authority.contains(':') {
        Some(authority.to_string())
    } else if url.starts_with("https://") {
        Some(format!("{authority}:443"))
    } else {
        Some(format!("{authority}:80"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config(method: HealthCheckMethod) -> HealthCheckConfig {
        HealthCheckConfig {
            interval_seconds: 30,
            timeout_seconds: 5,
            consecutive_failures_threshold: 3,
            method,
        }
    }

    #[test]
    fn builder_accessors() {
        let config = default_config(HealthCheckMethod::ConnectionPing);
        let builder = HealthCheckBuilder::new(config.clone());
        assert_eq!(builder.interval(), Duration::from_secs(30));
        assert_eq!(builder.timeout(), Duration::from_secs(5));
        assert_eq!(builder.failure_threshold(), 3);
        assert_eq!(builder.config(), &config);
    }

    #[tokio::test]
    async fn connection_ping_returns_healthy() {
        let builder = HealthCheckBuilder::new(default_config(HealthCheckMethod::ConnectionPing));
        let status = builder.check().await;
        assert_eq!(status, ComponentHealthStatus::Healthy);
    }

    #[tokio::test]
    async fn capability_probe_returns_healthy() {
        let builder = HealthCheckBuilder::new(default_config(HealthCheckMethod::CapabilityProbe));
        let status = builder.check().await;
        assert_eq!(status, ComponentHealthStatus::Healthy);
    }

    #[tokio::test]
    async fn timeout_returns_unhealthy() {
        let config = HealthCheckConfig {
            interval_seconds: 30,
            timeout_seconds: 0, // immediate timeout
            consecutive_failures_threshold: 3,
            method: HealthCheckMethod::HttpGet {
                url: "http://192.0.2.1:1".to_string(), // non-routable
            },
        };
        let builder = HealthCheckBuilder::new(config);
        let status = builder.check().await;
        assert!(
            matches!(status, ComponentHealthStatus::Unhealthy { .. }),
            "Expected Unhealthy on timeout, got: {:?}",
            status
        );
    }

    #[test]
    fn parse_host_port_http() {
        assert_eq!(
            parse_host_port("http://localhost:8080/health"),
            Some("localhost:8080".to_string())
        );
    }

    #[test]
    fn parse_host_port_https_default() {
        assert_eq!(
            parse_host_port("https://example.com/health"),
            Some("example.com:443".to_string())
        );
    }

    #[test]
    fn parse_host_port_http_default() {
        assert_eq!(
            parse_host_port("http://example.com/health"),
            Some("example.com:80".to_string())
        );
    }
}

// ---------------------------------------------------------------------------
// Health reporting types for component metrics (Req 43.1)
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Health status of a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Resource usage snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub disk_bytes: Option<u64>,
}

/// Structured health summary for a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub status: HealthStatus,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub dependencies_status: HashMap<String, HealthStatus>,
    pub resource_usage: ResourceUsage,
}

/// Definition of a metric emitted by a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub name: String,
    pub description: String,
    pub unit: String,
}

/// Aggregated metrics snapshot for a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub component_id: String,
    pub component_type: crate::SystemComponentType,
    pub request_count: u64,
    pub error_count: u64,
    pub latency_p50_ms: f64,
    pub latency_p99_ms: f64,
    pub custom_metrics: HashMap<String, serde_json::Value>,
}

/// Trait for components that report health metrics.
pub trait ComponentHealthReporter {
    fn report_metrics(&self) -> ComponentMetrics;
    fn metric_definitions(&self) -> Vec<MetricDefinition>;
    fn health_summary(&self) -> HealthSummary;
}

/// Collector for standard component metrics (request count, error count, latency).
pub struct MetricsCollector {
    component_id: String,
    component_type: crate::SystemComponentType,
    request_count: AtomicU64,
    error_count: AtomicU64,
    latencies: std::sync::Mutex<Vec<f64>>,
}

impl MetricsCollector {
    /// Create a new metrics collector for the given component.
    pub fn new(component_id: &str, component_type: crate::SystemComponentType) -> Self {
        Self {
            component_id: component_id.to_string(),
            component_type,
            request_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            latencies: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Record a successful request with the given latency in milliseconds.
    pub fn record_request(&self, latency_ms: f64) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut latencies) = self.latencies.lock() {
            latencies.push(latency_ms);
        }
    }

    /// Record an error.
    pub fn record_error(&self, _code: &str) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Take a snapshot of the current metrics.
    pub fn snapshot(&self) -> ComponentMetrics {
        let latencies = self.latencies.lock().map(|l| l.clone()).unwrap_or_default();
        let (p50, p99) = percentiles(&latencies);

        ComponentMetrics {
            component_id: self.component_id.clone(),
            component_type: self.component_type,
            request_count: self.request_count.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            latency_p50_ms: p50,
            latency_p99_ms: p99,
            custom_metrics: HashMap::new(),
        }
    }
}

/// Compute p50 and p99 percentiles from a slice of latencies.
fn percentiles(latencies: &[f64]) -> (f64, f64) {
    if latencies.is_empty() {
        return (0.0, 0.0);
    }
    let mut sorted = latencies.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p50_idx = (sorted.len() as f64 * 0.50) as usize;
    let p99_idx = (sorted.len() as f64 * 0.99) as usize;
    let p50 = sorted.get(p50_idx.min(sorted.len() - 1)).copied().unwrap_or(0.0);
    let p99 = sorted.get(p99_idx.min(sorted.len() - 1)).copied().unwrap_or(0.0);
    (p50, p99)
}
