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
