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
/// future that resolves to `Result<(), Box<dyn Error>>`.
type InitShutdownFn = Box<
    dyn FnMut() -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

/// Closure type for `health_check` — `Fn` returning a pinned future that
/// resolves to [`ComponentHealthStatus`].
type HealthCheckFn = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = ComponentHealthStatus> + Send>> + Send + Sync,
>;

/// Closure type for `config_schema` — `Fn` returning an optional JSON schema.
type ConfigSchemaFn = Box<dyn Fn() -> Option<serde_json::Value> + Send + Sync>;

/// Closure type for `current_config` — `Fn` returning the current config as JSON.
type CurrentConfigFn = Box<dyn Fn() -> Option<serde_json::Value> + Send + Sync>;

/// Closure type for `apply_config` — `FnMut` returning a pinned future that
/// resolves to `Result<(), Box<dyn Error>>`.
type ApplyConfigFn = Box<
    dyn FnMut(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
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
    config_schema_fn: Option<ConfigSchemaFn>,
    current_config_fn: Option<CurrentConfigFn>,
    apply_config_fn: Option<ApplyConfigFn>,
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
            config_schema_fn: None,
            current_config_fn: None,
            apply_config_fn: None,
        }
    }

    /// Set the initialization closure.
    ///
    /// The closure is called once during agent startup. It must return a
    /// pinned future resolving to `Result<()>`.
    pub fn on_initialize<F>(mut self, f: F) -> Self
    where
        F: FnMut() -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>
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
        F: FnMut() -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.shutdown_fn = Some(Box::new(f));
        self
    }

    /// Set the config-schema closure (optional).
    ///
    /// When provided, the built component's `config_schema()` delegates to
    /// this closure. When omitted, the trait default (`None`) is used.
    pub fn on_config_schema<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Option<serde_json::Value> + Send + Sync + 'static,
    {
        self.config_schema_fn = Some(Box::new(f));
        self
    }

    /// Set the current-config closure (optional).
    ///
    /// When provided, the built component's `current_config()` delegates to
    /// this closure. When omitted, the trait default (`None`) is used.
    pub fn on_current_config<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Option<serde_json::Value> + Send + Sync + 'static,
    {
        self.current_config_fn = Some(Box::new(f));
        self
    }

    /// Set the apply-config closure (optional).
    ///
    /// When provided, the built component's `apply_config()` delegates to
    /// this closure. When omitted, the trait default (returns `Err`) is used.
    pub fn on_apply_config<F>(mut self, f: F) -> Self
    where
        F: FnMut(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.apply_config_fn = Some(Box::new(f));
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
            config_schema_fn: self.config_schema_fn,
            current_config_fn: self.current_config_fn,
            apply_config_fn: self.apply_config_fn,
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
    config_schema_fn: Option<ConfigSchemaFn>,
    current_config_fn: Option<CurrentConfigFn>,
    apply_config_fn: Option<ApplyConfigFn>,
}

#[async_trait]
impl SystemComponent for ClosureComponent {
    fn component_name(&self) -> &str {
        &self.name
    }

    fn component_type(&self) -> SystemComponentType {
        self.component_type
    }

    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        (self.initialize_fn)().await
    }

    async fn health_check(&self) -> ComponentHealthStatus {
        (self.health_fn)().await
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        (self.shutdown_fn)().await
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        match &self.config_schema_fn {
            Some(f) => f(),
            None => None, // trait default
        }
    }

    fn current_config(&self) -> Option<serde_json::Value> {
        match &self.current_config_fn {
            Some(f) => f(),
            None => None, // trait default
        }
    }

    async fn apply_config(
        &mut self,
        config: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &mut self.apply_config_fn {
            Some(f) => f(config).await,
            None => Err("Configuration not supported by this component".into()),
        }
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

    #[tokio::test]
    async fn config_defaults_when_no_closures() {
        let mut component =
            SystemComponentBuilder::new("no-config", SystemComponentType::Cache)
                .on_initialize(|| Box::pin(async { Ok(()) }))
                .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
                .on_shutdown(|| Box::pin(async { Ok(()) }))
                .build()
                .unwrap();

        assert!(component.config_schema().is_none());
        assert!(component.current_config().is_none());
        let result = component.apply_config(serde_json::json!({"key": "val"})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not supported"));
    }

    #[tokio::test]
    async fn config_closures_are_called() {
        let schema = serde_json::json!({"type": "object"});
        let config = serde_json::json!({"enabled": true});
        let schema_clone = schema.clone();
        let config_clone = config.clone();

        let mut component =
            SystemComponentBuilder::new("configurable", SystemComponentType::Cache)
                .on_initialize(|| Box::pin(async { Ok(()) }))
                .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
                .on_shutdown(|| Box::pin(async { Ok(()) }))
                .on_config_schema(move || Some(schema_clone.clone()))
                .on_current_config(move || Some(config_clone.clone()))
                .on_apply_config(|_cfg| Box::pin(async { Ok(()) }))
                .build()
                .unwrap();

        assert_eq!(component.config_schema(), Some(schema));
        assert_eq!(component.current_config(), Some(config));
        assert!(component.apply_config(serde_json::json!({"new": "val"})).await.is_ok());
    }

    #[tokio::test]
    async fn apply_config_closure_receives_value() {
        use std::sync::{Arc, Mutex};

        let received = Arc::new(Mutex::new(None));
        let received_clone = received.clone();

        let mut component =
            SystemComponentBuilder::new("apply-test", SystemComponentType::Cache)
                .on_initialize(|| Box::pin(async { Ok(()) }))
                .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
                .on_shutdown(|| Box::pin(async { Ok(()) }))
                .on_apply_config(move |cfg| {
                    let r = received_clone.clone();
                    Box::pin(async move {
                        *r.lock().unwrap() = Some(cfg);
                        Ok(())
                    })
                })
                .build()
                .unwrap();

        let input = serde_json::json!({"timeout": 30});
        component.apply_config(input.clone()).await.unwrap();
        assert_eq!(*received.lock().unwrap(), Some(input));
    }

    #[tokio::test]
    async fn apply_config_closure_can_return_error() {
        let mut component =
            SystemComponentBuilder::new("err-test", SystemComponentType::Cache)
                .on_initialize(|| Box::pin(async { Ok(()) }))
                .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
                .on_shutdown(|| Box::pin(async { Ok(()) }))
                .on_apply_config(|_cfg| Box::pin(async {
                    Err("VALIDATION_FAILED: bad value".into())
                }))
                .build()
                .unwrap();

        let result = component.apply_config(serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("VALIDATION_FAILED"));
    }
}
