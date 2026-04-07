//! Sandbox violation types for process isolation reporting.
//!
//! This module defines the data types used to report sandbox violations
//! detected at runtime. Only the violation reporting types are included
//! here — the `ProcessSandbox` struct and platform-specific enforcement
//! logic stay in the agent crate.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SandboxViolation
// ---------------------------------------------------------------------------

/// A recorded sandbox violation detected at runtime.
///
/// Emitted when a sandboxed child process exceeds its configured limits
/// (output size, memory, timeout) or attempts to access restricted resources
/// (filesystem paths, environment variables, network).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxViolation {
    /// Provider or skill that owns the violating process.
    pub provider_id: String,
    /// Category of the violation.
    pub violation_type: SandboxViolationType,
    /// Human-readable detail about the violation.
    pub details: String,
    /// When the violation was detected.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// SandboxViolationType
// ---------------------------------------------------------------------------

/// The kind of sandbox violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxViolationType {
    PathAccess,
    EnvAccess,
    NetworkAccess,
    OutputExceeded,
    TimeoutExceeded,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // -- Unit tests -------------------------------------------------------

    #[test]
    fn sandbox_violation_serde_unit() {
        let v = SandboxViolation {
            provider_id: "skill-1".to_string(),
            violation_type: SandboxViolationType::OutputExceeded,
            details: "stdout exceeded 1048576 bytes".to_string(),
            timestamp: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: SandboxViolation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider_id, v.provider_id);
        assert_eq!(back.violation_type, v.violation_type);
        assert_eq!(back.details, v.details);
        assert_eq!(back.timestamp, v.timestamp);
    }

    #[test]
    fn sandbox_violation_type_all_variants_serialize() {
        let variants = vec![
            SandboxViolationType::PathAccess,
            SandboxViolationType::EnvAccess,
            SandboxViolationType::NetworkAccess,
            SandboxViolationType::OutputExceeded,
            SandboxViolationType::TimeoutExceeded,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: SandboxViolationType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    // -- Proptest strategies ----------------------------------------------

    fn arb_sandbox_violation_type() -> impl Strategy<Value = SandboxViolationType> {
        prop_oneof![
            Just(SandboxViolationType::PathAccess),
            Just(SandboxViolationType::EnvAccess),
            Just(SandboxViolationType::NetworkAccess),
            Just(SandboxViolationType::OutputExceeded),
            Just(SandboxViolationType::TimeoutExceeded),
        ]
    }

    fn arb_sandbox_violation() -> impl Strategy<Value = SandboxViolation> {
        (
            "\\w{1,30}",
            arb_sandbox_violation_type(),
            "\\w{0,50}",
            // Generate a timestamp within a reasonable range
            (0i64..=4_102_444_800i64),
        )
            .prop_map(|(provider_id, violation_type, details, secs)| {
                let timestamp = chrono::DateTime::from_timestamp(secs, 0)
                    .unwrap_or_else(chrono::Utc::now);
                SandboxViolation {
                    provider_id,
                    violation_type,
                    details,
                    timestamp,
                }
            })
    }

    // -- Property tests ---------------------------------------------------

    proptest! {
        /// Property 1: Serde JSON round-trip for sandbox violation types
        ///
        /// **Validates: Requirements 8.1, 13.1**
        ///
        /// For any valid `SandboxViolation`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_sandbox_violation(v in arb_sandbox_violation()) {
            let json = serde_json::to_string(&v).unwrap();
            let back: SandboxViolation = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, v);
        }

        /// Property 1: Serde JSON round-trip for sandbox violation types
        ///
        /// **Validates: Requirements 8.1, 13.1**
        ///
        /// For any valid `SandboxViolationType`, serializing to JSON and
        /// deserializing back produces the original value.
        #[test]
        fn serde_round_trip_sandbox_violation_type(vt in arb_sandbox_violation_type()) {
            let json = serde_json::to_string(&vt).unwrap();
            let back: SandboxViolationType = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, vt);
        }
    }
}
