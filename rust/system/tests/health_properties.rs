//! Property-based tests for System SDK health check.
//!
//! **Property 8: Health check timeout returns failure status rather than blocking**
//! **Validates: Requirements 7.3**
//!
//! **Property 23: HealthCheckBuilder produces config-matching implementation**
//! **Validates: Requirements 7.1**

use std::time::Duration;

use lifesavor_system_sdk::health::HealthCheckBuilder;
use lifesavor_system_sdk::{ComponentHealthStatus, HealthCheckConfig, HealthCheckMethod};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy for generating arbitrary `HealthCheckMethod` variants.
fn arb_health_check_method() -> impl Strategy<Value = HealthCheckMethod> {
    prop_oneof![
        Just(HealthCheckMethod::ConnectionPing),
        Just(HealthCheckMethod::CapabilityProbe),
    ]
}

/// Strategy for generating arbitrary `HealthCheckConfig` values.
fn arb_health_check_config() -> impl Strategy<Value = HealthCheckConfig> {
    (
        1u64..=3600,  // interval_seconds
        1u64..=300,   // timeout_seconds
        1u32..=100,   // consecutive_failures_threshold
        arb_health_check_method(),
    )
        .prop_map(|(interval, timeout, threshold, method)| HealthCheckConfig {
            interval_seconds: interval,
            timeout_seconds: timeout,
            consecutive_failures_threshold: threshold,
            method,
        })
}

// ---------------------------------------------------------------------------
// Property 23: HealthCheckBuilder produces config-matching implementation
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 23: HealthCheckBuilder produces config-matching implementation**
    ///
    /// For any valid HealthCheckConfig, the builder correctly stores and returns
    /// the config parameters (interval, timeout, failure_threshold).
    ///
    /// **Validates: Requirements 7.1**
    #[test]
    fn health_check_builder_matches_config(config in arb_health_check_config()) {
        let builder = HealthCheckBuilder::new(config.clone());

        prop_assert_eq!(
            builder.interval(),
            Duration::from_secs(config.interval_seconds),
            "interval() should match config.interval_seconds"
        );
        prop_assert_eq!(
            builder.timeout(),
            Duration::from_secs(config.timeout_seconds),
            "timeout() should match config.timeout_seconds"
        );
        prop_assert_eq!(
            builder.failure_threshold(),
            config.consecutive_failures_threshold,
            "failure_threshold() should match config.consecutive_failures_threshold"
        );
        prop_assert_eq!(
            builder.config(),
            &config,
            "config() should return the original config"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 8: Health check timeout returns failure status rather than blocking
// ---------------------------------------------------------------------------

/// **Property 8: Health check timeout returns failure status rather than blocking**
///
/// When a health check exceeds the configured timeout, it returns
/// `ComponentHealthStatus::Unhealthy` rather than blocking indefinitely.
/// We use a zero-second timeout with a non-routable address to guarantee
/// the probe exceeds the timeout.
///
/// **Validates: Requirements 7.3**
#[test]
fn health_check_timeout_returns_unhealthy_not_blocking() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut runner = proptest::test_runner::TestRunner::default();

    runner
        .run(&(1u32..=50u32), |threshold| {
            rt.block_on(async {
                let config = HealthCheckConfig {
                    interval_seconds: 30,
                    timeout_seconds: 0, // immediate timeout
                    consecutive_failures_threshold: threshold,
                    method: HealthCheckMethod::HttpGet {
                        url: "http://192.0.2.1:1".to_string(), // non-routable
                    },
                };
                let builder = HealthCheckBuilder::new(config);

                // The check must complete (not block) and return Unhealthy
                let status = tokio::time::timeout(Duration::from_secs(5), builder.check())
                    .await
                    .expect("health check should not block for more than 5 seconds");

                prop_assert!(
                    matches!(status, ComponentHealthStatus::Unhealthy { .. }),
                    "Expected Unhealthy on timeout, got: {:?}",
                    status
                );
                Ok(())
            })?;

            Ok(())
        })
        .unwrap();
}
