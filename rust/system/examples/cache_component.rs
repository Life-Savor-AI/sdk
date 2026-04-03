//! Minimal cache system component example.
//!
//! Demonstrates building a cache system component with get/set/delete
//! operations using the `SystemComponentBuilder`, plus a health check.
//!
//! Run with: `cargo run --example cache_component`

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lifesavor_system_sdk::prelude::*;
use lifesavor_system_sdk::builder::SystemComponentBuilder;
use lifesavor_system_sdk::testing::MockAgentContext;

/// Simple in-memory cache store shared across async tasks.
#[derive(Clone, Default)]
struct CacheStore {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl CacheStore {
    fn new() -> Self {
        Self::default()
    }

    /// Insert or update a cache entry.
    #[instrument(skip(self))]
    fn set(&self, key: &str, value: &str) {
        self.inner
            .lock()
            .expect("lock poisoned")
            .insert(key.to_string(), value.to_string());
        info!(key, "Cache SET");
    }

    /// Retrieve a cache entry.
    #[instrument(skip(self))]
    fn get(&self, key: &str) -> Option<String> {
        let val = self.inner.lock().expect("lock poisoned").get(key).cloned();
        info!(key, found = val.is_some(), "Cache GET");
        val
    }

    /// Remove a cache entry, returning the previous value if present.
    #[instrument(skip(self))]
    fn delete(&self, key: &str) -> Option<String> {
        let val = self.inner.lock().expect("lock poisoned").remove(key);
        info!(key, found = val.is_some(), "Cache DELETE");
        val
    }

    /// Return the number of entries (used for health check).
    fn len(&self) -> usize {
        self.inner.lock().expect("lock poisoned").len()
    }
}

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Building cache system component");

    let store = CacheStore::new();
    let health_store = store.clone();

    let component = SystemComponentBuilder::new("cache", SystemComponentType::Cache)
        .on_initialize(|| {
            Box::pin(async {
                info!("Cache component initialized");
                Ok(())
            })
        })
        .on_health_check(move || {
            let s = health_store.clone();
            Box::pin(async move {
                // A real component might check memory pressure or backend
                // connectivity. Here we just report healthy with entry count.
                info!(entries = s.len(), "Cache health check");
                ComponentHealthStatus::Healthy
            })
        })
        .on_shutdown(|| {
            Box::pin(async {
                info!("Cache component shutting down");
                Ok(())
            })
        })
        .build()
        .expect("Cache component builder should succeed");

    // Run through the mock lifecycle.
    let mut ctx = MockAgentContext::new();
    ctx.register(component);
    ctx.initialize().await.expect("initialize should succeed");

    // Demonstrate cache operations.
    store.set("user:1", "Alice");
    store.set("user:2", "Bob");

    let alice = store.get("user:1");
    info!(?alice, "Retrieved user:1");

    let removed = store.delete("user:2");
    info!(?removed, "Deleted user:2");

    let missing = store.get("user:2");
    info!(?missing, "user:2 after delete");

    // Health check.
    let health = ctx.health_check().await.expect("health check should succeed");
    info!(?health, "Health check result");

    ctx.shutdown().await.expect("shutdown should succeed");
    info!("Cache example complete");
}
