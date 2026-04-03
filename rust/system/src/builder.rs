//! Builder for constructing [`SystemComponent`] implementations from closures.
//!
//! The [`SystemComponentBuilder`] provides an ergonomic way to create system
//! components without manually implementing the [`SystemComponent`] trait.
//!
//! # Example
//!
//! ```rust,ignore
//! use lifesavor_system_sdk::prelude::*;
//! use lifesavor_system_sdk::builder::SystemComponentBuilder;
//!
//! let component = SystemComponentBuilder::new("my-cache", SystemComponentType::Cache)
//!     .on_initialize(|| Box::pin(async { Ok(()) }))
//!     .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
//!     .on_shutdown(|| Box::pin(async { Ok(()) }))
//!     .build()
//!     .expect("all fields provided");
//! ```

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;

use crate::{
    ComponentHealthStatus, SystemComponent, SystemComponentType,
};
use crate::error::SystemSdkError;

// ---------------------------------------------------------------------------
// Type aliases for the closure fields
// ---------------------------------------------------------------------------

/// Closure type for `initialize` and `shutdown` — `FnMut` returning a pinned
/// future that resolves to `Result<(), AgentError>`.
type InitShutdownFn = Box<
    dyn FnMut() -> Pin<Box<dyn Future<Output = lifesavor_agent::error::Result<()>> + Send>>
        + Send
        + Sync,
>;

/// Closure type for `health_check` — `Fn` returning a pinned future that
/// resolves to [`ComponentHealthStatus`].
type HealthCheckFn = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = ComponentHealthStatus> + Send>> + Send + Sync,
>;

// ---------------------------------------------------------------------------
// SystemComponentBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing a [`SystemComponent`] from closures.
///
/// Accepts a component name, [`SystemComponentType`], and closures for
/// `initialize`, `health_check`, and `shutdown`. Calling [`build()`](Self::build)
/// validates that all required fields are present and returns a boxed
/// `SystemComponent` implementation.
pub struct SystemComponentBuilder {
    name: String,
    component_type: SystemComponentType,
    initialize_fn: Option<InitShutdownFn>,
    health_fn: Option<HealthCheckFn>,
    shutdown_fn: Option<InitShutdownFn>,
}

impl SystemComponentBuilder {
    /// Create a new builder with the given component name and type.
    pub fn new(name: &str, component_type: SystemComponentType) -> Self {
        Self {
            name: name.to_owned(),
            component_type,
            initialize_fn: None,
            health_fn: None,
            shutdown_fn: None,
        }
    }

    /// Set the initialization closure.
    ///
    /// The closure is called once during agent startup. It must return a
    /// pinned future resolving to `Result<()>`.
    pub fn on_initialize<F>(mut self, f: F) -> Self
    where
        F: FnMut() -> Pin<Box<dyn Future<Output = lifesavor_agent::error::Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.initialize_fn = Some(Box::new(f));
        self
    }

    /// Set the health-check closure.
    ///
    /// The closure is called periodically by the Health Monitor. It must
    /// return a pinned future resolving to [`ComponentHealthStatus`].
    pub fn on_health_check<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Pin<Box<dyn Future<Output = ComponentHealthStatus> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.health_fn = Some(Box::new(f));
        self
    }

    /// Set the shutdown closure.
    ///
    /// The closure is called once during agent shutdown. It must return a
    /// pinned future resolving to `Result<()>`.
    pub fn on_shutdown<F>(mut self, f: F) -> Self
    where
        F: FnMut() -> Pin<Box<dyn Future<Output = lifesavor_agent::error::Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.shutdown_fn = Some(Box::new(f));
        self
    }

    /// Consume the builder and produce a boxed [`SystemComponent`].
    ///
    /// Returns an error if the component name is empty or any closure is
    /// missing.
    pub fn build(self) -> Result<Box<dyn SystemComponent>, SystemSdkError> {
        if self.name.trim().is_empty() {
            return Err(SystemSdkError::InitFailed(
                "component name must not be empty".into(),
            ));
        }
        let initialize_fn = self.initialize_fn.ok_or_else(|| {
            SystemSdkError::InitFailed("initialize closure is required".into())
        })?;
        let health_fn = self.health_fn.ok_or_else(|| {
            SystemSdkError::InitFailed("health_check closure is required".into())
        })?;
        let shutdown_fn = self.shutdown_fn.ok_or_else(|| {
            SystemSdkError::InitFailed("shutdown closure is required".into())
        })?;

        Ok(Box::new(ClosureComponent {
            name: self.name,
            component_type: self.component_type,
            initialize_fn,
            health_fn,
            shutdown_fn,
        }))
    }
}

// ---------------------------------------------------------------------------
// ClosureComponent — the concrete struct produced by the builder
// ---------------------------------------------------------------------------

/// Internal concrete type that implements [`SystemComponent`] by delegating
/// to the closures provided via [`SystemComponentBuilder`].
struct ClosureComponent {
    name: String,
    component_type: SystemComponentType,
    initialize_fn: InitShutdownFn,
    health_fn: HealthCheckFn,
    shutdown_fn: InitShutdownFn,
}

#[async_trait]
impl SystemComponent for ClosureComponent {
    fn component_name(&self) -> &str {
        &self.name
    }

    fn component_type(&self) -> SystemComponentType {
        self.component_type
    }

    async fn initialize(&mut self) -> lifesavor_agent::error::Result<()> {
        (self.initialize_fn)().await
    }

    async fn health_check(&self) -> ComponentHealthStatus {
        (self.health_fn)().await
    }

    async fn shutdown(&mut self) -> lifesavor_agent::error::Result<()> {
        (self.shutdown_fn)().await
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn build_with_all_fields_succeeds() {
        let result = SystemComponentBuilder::new("test-cache", SystemComponentType::Cache)
            .on_initialize(|| Box::pin(async { Ok(()) }))
            .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
            .on_shutdown(|| Box::pin(async { Ok(()) }))
            .build();

        assert!(result.is_ok());
        let component = result.unwrap();
        assert_eq!(component.component_name(), "test-cache");
        assert_eq!(component.component_type(), SystemComponentType::Cache);
    }

    #[tokio::test]
    async fn build_runs_lifecycle() {
        let mut component =
            SystemComponentBuilder::new("lifecycle", SystemComponentType::Cache)
                .on_initialize(|| Box::pin(async { Ok(()) }))
                .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
                .on_shutdown(|| Box::pin(async { Ok(()) }))
                .build()
                .unwrap();

        assert!(component.initialize().await.is_ok());
        assert_eq!(component.health_check().await, ComponentHealthStatus::Healthy);
        assert!(component.shutdown().await.is_ok());
    }

    /// Helper to extract the error from a build result (since `Box<dyn
    /// SystemComponent>` doesn't implement `Debug`, we can't use `unwrap_err`).
    fn expect_err(result: Result<Box<dyn SystemComponent>, SystemSdkError>) -> SystemSdkError {
        match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err, got Ok"),
        }
    }

    #[test]
    fn build_rejects_empty_name() {
        let result = SystemComponentBuilder::new("", SystemComponentType::Cache)
            .on_initialize(|| Box::pin(async { Ok(()) }))
            .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
            .on_shutdown(|| Box::pin(async { Ok(()) }))
            .build();

        let err = expect_err(result);
        assert!(err.to_string().contains("name must not be empty"));
    }

    #[test]
    fn build_rejects_whitespace_only_name() {
        let result = SystemComponentBuilder::new("   ", SystemComponentType::Cache)
            .on_initialize(|| Box::pin(async { Ok(()) }))
            .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
            .on_shutdown(|| Box::pin(async { Ok(()) }))
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn build_rejects_missing_initialize() {
        let err = expect_err(
            SystemComponentBuilder::new("test", SystemComponentType::Cache)
                .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
                .on_shutdown(|| Box::pin(async { Ok(()) }))
                .build(),
        );
        assert!(err.to_string().contains("initialize"));
    }

    #[test]
    fn build_rejects_missing_health_check() {
        let err = expect_err(
            SystemComponentBuilder::new("test", SystemComponentType::Cache)
                .on_initialize(|| Box::pin(async { Ok(()) }))
                .on_shutdown(|| Box::pin(async { Ok(()) }))
                .build(),
        );
        assert!(err.to_string().contains("health_check"));
    }

    #[test]
    fn build_rejects_missing_shutdown() {
        let err = expect_err(
            SystemComponentBuilder::new("test", SystemComponentType::Cache)
                .on_initialize(|| Box::pin(async { Ok(()) }))
                .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
                .build(),
        );
        assert!(err.to_string().contains("shutdown"));
    }
}
