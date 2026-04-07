//! System component types shared between the agent and SDK.
//!
//! This module defines the core trait and data types for system components.
//! The `SystemComponent` trait uses a generic error type
//! (`Box<dyn std::error::Error + Send + Sync>`) so that implementations are
//! not coupled to any agent-specific error type.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// SystemComponentType
// ---------------------------------------------------------------------------

/// Enumerates the kinds of system components the agent supports.
///
/// Unlike the agent-internal version, all variants are always available
/// (no feature gates) so that SDK consumers can reference any component type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemComponentType {
    Tts,
    Stt,
    Cache,
    Identity,
    FileStorage,
    Messaging,
    Calendar,
    DeviceControl,
    MediaProcessing,
    UserNotifications,
    Llm,
    VectorStore,
}

impl fmt::Display for SystemComponentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tts => write!(f, "tts"),
            Self::Stt => write!(f, "stt"),
            Self::Cache => write!(f, "cache"),
            Self::Identity => write!(f, "identity"),
            Self::FileStorage => write!(f, "file_storage"),
            Self::Messaging => write!(f, "messaging"),
            Self::Calendar => write!(f, "calendar"),
            Self::DeviceControl => write!(f, "device_control"),
            Self::MediaProcessing => write!(f, "media_processing"),
            Self::UserNotifications => write!(f, "user_notifications"),
            Self::Llm => write!(f, "llm"),
            Self::VectorStore => write!(f, "vector_store"),
        }
    }
}

// ---------------------------------------------------------------------------
// ComponentHealthStatus
// ---------------------------------------------------------------------------

/// Health status reported by a system component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentHealthStatus {
    Healthy,
    Degraded { details: String },
    Unhealthy { details: String },
    Unknown,
}

impl fmt::Display for ComponentHealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded { details } => write!(f, "degraded: {details}"),
            Self::Unhealthy { details } => write!(f, "unhealthy: {details}"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// SystemComponent trait
// ---------------------------------------------------------------------------

/// The core trait that all system components implement.
///
/// Uses a generic error type so implementations are not coupled to any
/// agent-specific error enum. The agent wraps errors into its own type
/// at the call boundary.
#[async_trait]
pub trait SystemComponent: Send + Sync {
    /// Human-readable name (e.g. `"tts"`, `"cache"`).
    fn component_name(&self) -> &str;

    /// The component type enum variant.
    fn component_type(&self) -> SystemComponentType;

    /// Initialise the component.
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Return the current health status.
    async fn health_check(&self) -> ComponentHealthStatus;

    /// Gracefully shut down the component.
    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

// ---------------------------------------------------------------------------
// SystemComponentInfo
// ---------------------------------------------------------------------------

/// Metadata about a registered system component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemComponentInfo {
    /// Component name (matches `SystemComponent::component_name()`).
    pub name: String,
    /// Component type.
    pub component_type: SystemComponentType,
    /// Last observed health status.
    pub health: ComponentHealthStatus,
    /// When the last health check was performed.
    pub last_health_check: Option<DateTime<Utc>>,
    /// When the component was registered.
    pub registered_at: DateTime<Utc>,
    /// Capabilities advertised by the component (free-form tags).
    pub capabilities: Vec<String>,
}

// ---------------------------------------------------------------------------
// Object safety compile-time check
// ---------------------------------------------------------------------------

/// Compile-time assertion that `SystemComponent` is object-safe.
const _: () = {
    fn _assert_object_safe(_: &dyn SystemComponent) {}
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // -- Unit tests -------------------------------------------------------

    #[test]
    fn component_type_display() {
        assert_eq!(SystemComponentType::Tts.to_string(), "tts");
        assert_eq!(SystemComponentType::Stt.to_string(), "stt");
        assert_eq!(SystemComponentType::Cache.to_string(), "cache");
        assert_eq!(SystemComponentType::Identity.to_string(), "identity");
        assert_eq!(SystemComponentType::FileStorage.to_string(), "file_storage");
        assert_eq!(SystemComponentType::Messaging.to_string(), "messaging");
        assert_eq!(SystemComponentType::Calendar.to_string(), "calendar");
        assert_eq!(SystemComponentType::DeviceControl.to_string(), "device_control");
        assert_eq!(SystemComponentType::MediaProcessing.to_string(), "media_processing");
        assert_eq!(SystemComponentType::UserNotifications.to_string(), "user_notifications");
        assert_eq!(SystemComponentType::Llm.to_string(), "llm");
        assert_eq!(SystemComponentType::VectorStore.to_string(), "vector_store");
    }

    #[test]
    fn health_status_display() {
        assert_eq!(ComponentHealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(
            ComponentHealthStatus::Degraded { details: "slow".into() }.to_string(),
            "degraded: slow"
        );
        assert_eq!(
            ComponentHealthStatus::Unhealthy { details: "down".into() }.to_string(),
            "unhealthy: down"
        );
        assert_eq!(ComponentHealthStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn component_type_serde_round_trip_unit() {
        let ct = SystemComponentType::Cache;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"cache\"");
        let back: SystemComponentType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn health_status_serde_round_trip_unit() {
        let h = ComponentHealthStatus::Degraded { details: "slow".into() };
        let json = serde_json::to_string(&h).unwrap();
        let back: ComponentHealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn system_component_info_serde_round_trip_unit() {
        let info = SystemComponentInfo {
            name: "tts".into(),
            component_type: SystemComponentType::Tts,
            health: ComponentHealthStatus::Healthy,
            last_health_check: Some(Utc::now()),
            registered_at: Utc::now(),
            capabilities: vec!["speak".into()],
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: SystemComponentInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, info.name);
        assert_eq!(back.component_type, info.component_type);
        assert_eq!(back.health, info.health);
        assert_eq!(back.capabilities, info.capabilities);
    }

    // -- Proptest strategies ----------------------------------------------

    fn arb_system_component_type() -> impl Strategy<Value = SystemComponentType> {
        prop_oneof![
            Just(SystemComponentType::Tts),
            Just(SystemComponentType::Stt),
            Just(SystemComponentType::Cache),
            Just(SystemComponentType::Identity),
            Just(SystemComponentType::FileStorage),
            Just(SystemComponentType::Messaging),
            Just(SystemComponentType::Calendar),
            Just(SystemComponentType::DeviceControl),
            Just(SystemComponentType::MediaProcessing),
            Just(SystemComponentType::UserNotifications),
            Just(SystemComponentType::Llm),
            Just(SystemComponentType::VectorStore),
        ]
    }

    fn arb_component_health_status() -> impl Strategy<Value = ComponentHealthStatus> {
        prop_oneof![
            Just(ComponentHealthStatus::Healthy),
            ".*".prop_map(|s: String| ComponentHealthStatus::Degraded { details: s }),
            ".*".prop_map(|s: String| ComponentHealthStatus::Unhealthy { details: s }),
            Just(ComponentHealthStatus::Unknown),
        ]
    }

    fn arb_system_component_info() -> impl Strategy<Value = SystemComponentInfo> {
        (
            "\\w+",
            arb_system_component_type(),
            arb_component_health_status(),
            any::<bool>(),
            proptest::collection::vec("\\w+", 0..5),
        )
            .prop_map(|(name, component_type, health, has_last_check, capabilities)| {
                SystemComponentInfo {
                    name,
                    component_type,
                    health,
                    last_health_check: if has_last_check { Some(Utc::now()) } else { None },
                    registered_at: Utc::now(),
                    capabilities,
                }
            })
    }

    // -- Property tests ---------------------------------------------------

    proptest! {
        /// Property 1: Serde JSON round-trip for system component types
        ///
        /// **Validates: Requirements 13.1**
        ///
        /// For any valid `SystemComponentType`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_system_component_type(ct in arb_system_component_type()) {
            let json = serde_json::to_string(&ct).unwrap();
            let back: SystemComponentType = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, ct);
        }

        /// Property 1: Serde JSON round-trip for ComponentHealthStatus
        ///
        /// **Validates: Requirements 13.1**
        ///
        /// For any valid `ComponentHealthStatus`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_component_health_status(h in arb_component_health_status()) {
            let json = serde_json::to_string(&h).unwrap();
            let back: ComponentHealthStatus = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, h);
        }

        /// Property 1: Serde JSON round-trip for SystemComponentInfo
        ///
        /// **Validates: Requirements 13.1**
        ///
        /// For any valid `SystemComponentInfo`, serializing to JSON and
        /// deserializing back produces a value with identical fields.
        #[test]
        fn serde_round_trip_system_component_info(info in arb_system_component_info()) {
            let json = serde_json::to_string(&info).unwrap();
            let back: SystemComponentInfo = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(&back.name, &info.name);
            prop_assert_eq!(back.component_type, info.component_type);
            prop_assert_eq!(&back.health, &info.health);
            prop_assert_eq!(&back.capabilities, &info.capabilities);
            prop_assert_eq!(back.last_health_check, info.last_health_check);
            prop_assert_eq!(back.registered_at, info.registered_at);
        }
    }
}
