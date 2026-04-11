//! Unified component declaration types.
//!
//! This module defines the [`ComponentDeclaration`] struct and its supporting
//! types ([`AccessRequest`], [`PermissionScope`], [`ToolSchema`]). These are
//! the canonical definitions shared across all crates — agent runtime, system
//! SDK, skill SDK, and individual component crates.

use serde::{Deserialize, Serialize};

use crate::system_component::SystemComponentType;

// ---------------------------------------------------------------------------
// ToolSchema
// ---------------------------------------------------------------------------

/// Schema describing a single tool exposed by a component or skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Unique tool name within this component.
    pub name: String,
    /// Human-readable description of the tool.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
    /// JSON Schema for the tool's output (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// PermissionScope
// ---------------------------------------------------------------------------

/// Permission scope for component access control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    /// Invoke tools/skills on the target.
    Invoke,
    /// Read from the target's memory namespace.
    MemoryRead,
    /// Write to the target's memory namespace.
    MemoryWrite,
    /// Administrative operations (register/deregister).
    Admin,
    /// Publish to a topic owned by the target component.
    TopicPublish,
    /// Subscribe to a topic owned by the target component.
    TopicSubscribe,
}

impl std::fmt::Display for PermissionScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invoke => f.write_str("invoke"),
            Self::MemoryRead => f.write_str("memory.read"),
            Self::MemoryWrite => f.write_str("memory.write"),
            Self::Admin => f.write_str("admin"),
            Self::TopicPublish => f.write_str("topic.publish"),
            Self::TopicSubscribe => f.write_str("topic.subscribe"),
        }
    }
}

// ---------------------------------------------------------------------------
// AccessRequest
// ---------------------------------------------------------------------------

/// A request for access to a specific target component with given scopes.
///
/// Part of a [`ComponentDeclaration`]'s `requested_access` list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequest {
    /// The component ID being requested for access.
    pub target_component_id: String,
    /// The permission scopes requested on the target.
    pub scopes: Vec<PermissionScope>,
}

// ---------------------------------------------------------------------------
// ComponentDeclaration
// ---------------------------------------------------------------------------

/// Unified declaration of a component's identity, capabilities, and access
/// requirements.
///
/// Submitted during component registration (compiled-in or external). This is
/// a declaration of intent — it does NOT grant any access by itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDeclaration {
    /// Unique identifier for this component.
    pub component_id: String,
    /// The type of system component.
    pub component_type: SystemComponentType,
    /// Optional instance ID for multi-instance component types.
    pub instance_id: Option<String>,
    /// Bridge operations this component exposes to others.
    pub exposed_operations: Vec<String>,
    /// MessageBus topics this component publishes to.
    pub publish_topics: Vec<String>,
    /// Access this component requests to other components/topics.
    pub requested_access: Vec<AccessRequest>,
    /// Vault keys this component requires.
    pub requested_vault_keys: Vec<String>,
    /// Tool schemas for MCP discovery.
    pub tool_schemas: Vec<ToolSchema>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_declaration_serde_round_trip() {
        let decl = ComponentDeclaration {
            component_id: "my-store".to_string(),
            component_type: SystemComponentType::MemoryStore,
            instance_id: Some("memory_store:my-store-v1".to_string()),
            exposed_operations: vec!["store".into(), "search".into()],
            publish_topics: vec!["memory_store.component.ready".into()],
            requested_access: vec![AccessRequest {
                target_component_id: "cache".to_string(),
                scopes: vec![PermissionScope::Invoke],
            }],
            requested_vault_keys: vec!["db-password".into()],
            tool_schemas: vec![ToolSchema {
                name: "store".to_string(),
                description: "Store a document".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
                output_schema: None,
            }],
        };

        let json = serde_json::to_string(&decl).unwrap();
        let back: ComponentDeclaration = serde_json::from_str(&json).unwrap();

        assert_eq!(back.component_id, "my-store");
        assert_eq!(back.component_type, SystemComponentType::MemoryStore);
        assert_eq!(back.instance_id, Some("memory_store:my-store-v1".to_string()));
        assert_eq!(back.exposed_operations, vec!["store", "search"]);
        assert_eq!(back.publish_topics, vec!["memory_store.component.ready"]);
        assert_eq!(back.requested_access.len(), 1);
        assert_eq!(back.requested_access[0].target_component_id, "cache");
        assert_eq!(back.requested_vault_keys, vec!["db-password"]);
        assert_eq!(back.tool_schemas.len(), 1);
        assert_eq!(back.tool_schemas[0].name, "store");
    }

    #[test]
    fn component_declaration_without_optional_fields() {
        let decl = ComponentDeclaration {
            component_id: "tts".to_string(),
            component_type: SystemComponentType::Tts,
            instance_id: None,
            exposed_operations: vec!["speak".into()],
            publish_topics: vec![],
            requested_access: vec![],
            requested_vault_keys: vec![],
            tool_schemas: vec![],
        };

        let json = serde_json::to_string(&decl).unwrap();
        let back: ComponentDeclaration = serde_json::from_str(&json).unwrap();

        assert_eq!(back.component_id, "tts");
        assert!(back.instance_id.is_none());
        assert!(back.tool_schemas.is_empty());
    }

    #[test]
    fn permission_scope_display() {
        assert_eq!(PermissionScope::Invoke.to_string(), "invoke");
        assert_eq!(PermissionScope::MemoryRead.to_string(), "memory.read");
        assert_eq!(PermissionScope::MemoryWrite.to_string(), "memory.write");
        assert_eq!(PermissionScope::Admin.to_string(), "admin");
        assert_eq!(PermissionScope::TopicPublish.to_string(), "topic.publish");
        assert_eq!(PermissionScope::TopicSubscribe.to_string(), "topic.subscribe");
    }

    #[test]
    fn permission_scope_serde_round_trip() {
        let scope = PermissionScope::MemoryRead;
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, "\"memory_read\"");
        let back: PermissionScope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, scope);
    }

    #[test]
    fn tool_schema_serde_round_trip() {
        let schema = ToolSchema {
            name: "search".to_string(),
            description: "Search documents".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                }
            }),
            output_schema: Some(serde_json::json!({"type": "array"})),
        };

        let json = serde_json::to_string(&schema).unwrap();
        let back: ToolSchema = serde_json::from_str(&json).unwrap();

        assert_eq!(back.name, "search");
        assert_eq!(back.description, "Search documents");
        assert!(back.output_schema.is_some());
    }

    #[test]
    fn access_request_serde_round_trip() {
        let req = AccessRequest {
            target_component_id: "cache".to_string(),
            scopes: vec![PermissionScope::Invoke, PermissionScope::MemoryRead],
        };

        let json = serde_json::to_string(&req).unwrap();
        let back: AccessRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(back.target_component_id, "cache");
        assert_eq!(back.scopes.len(), 2);
    }
}
