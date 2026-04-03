//! Health check builder for the Model SDK.
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

use crate::{HealthCheckConfig, HealthCheckMethod};
use lifesavor_agent::providers::skill_provider::HealthStatus;

/// Builder that constructs a runnable health check from a [`HealthCheckConfig`].
///
/// # Example
///
/// ```rust,ignore
/// use lifesavor_model_sdk::health::HealthCheckBuilder;
/// use lifesavor_model_sdk::{HealthCheckConfig, HealthCheckMethod};
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
    /// [`HealthStatus::Unhealthy`] rather than blocking.
    pub async fn check(&self) -> HealthStatus {
        let timeout_dur = self.timeout();

        match tokio::time::timeout(timeout_dur, self.run_probe()).await {
            Ok(status) => status,
            Err(_elapsed) => HealthStatus::Unhealthy {
                details: format!(
                    "Health check timed out after {}s",
                    self.config.timeout_seconds
                ),
            },
        }
    }

    /// Run the actual probe based on the configured method.
    async fn run_probe(&self) -> HealthStatus {
        match &self.config.method {
            HealthCheckMethod::HttpGet { url } => self.probe_http(url).await,
            HealthCheckMethod::ConnectionPing => self.probe_connection_ping().await,
            HealthCheckMethod::CapabilityProbe => self.probe_capability().await,
        }
    }

    /// HTTP GET probe — attempts a TCP connection to the URL's host and port.
    async fn probe_http(&self, url: &str) -> HealthStatus {
        match parse_host_port(url) {
            Some(addr) => match tokio::net::TcpStream::connect(&addr).await {
                Ok(_) => HealthStatus::Healthy,
                Err(e) => HealthStatus::Unhealthy {
                    details: format!("HTTP health check failed: {e}"),
                },
            },
            None => HealthStatus::Unhealthy {
                details: format!("Invalid health check URL: {url}"),
            },
        }
    }

    /// Connection ping probe — a lightweight connectivity check.
    ///
    /// Default implementation returns Healthy. Provider developers should
    /// wrap or extend this for real connectivity verification.
    async fn probe_connection_ping(&self) -> HealthStatus {
        HealthStatus::Healthy
    }

    /// Capability probe — verifies the provider can serve requests.
    ///
    /// Default implementation returns Healthy.
    async fn probe_capability(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

/// Parse a URL string into a `host:port` address for TCP connection.
fn parse_host_port(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

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
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn capability_probe_returns_healthy() {
        let builder = HealthCheckBuilder::new(default_config(HealthCheckMethod::CapabilityProbe));
        let status = builder.check().await;
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn timeout_returns_unhealthy() {
        let config = HealthCheckConfig {
            interval_seconds: 30,
            timeout_seconds: 0,
            consecutive_failures_threshold: 3,
            method: HealthCheckMethod::HttpGet {
                url: "http://192.0.2.1:1".to_string(),
            },
        };
        let builder = HealthCheckBuilder::new(config);
        let status = builder.check().await;
        assert!(
            matches!(status, HealthStatus::Unhealthy { .. }),
            "Expected Unhealthy on timeout, got: {:?}",
            status
        );
    }
}
