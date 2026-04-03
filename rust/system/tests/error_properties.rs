//! Property-based tests for `SystemSdkError`.
//!
//! **Validates: Requirements 9.2, 21.2, 21.3**

use lifesavor_system_sdk::error::SystemSdkError;
use lifesavor_system_sdk::{ErrorContext, Subsystem};
use proptest::prelude::*;

/// Strategy that generates an arbitrary `SystemSdkError` variant.
///
/// We use `prop_map` to construct errors lazily (SystemSdkError is not Clone).
/// ManifestValidationError is excluded since it requires agent-internal state.
fn arb_system_sdk_error() -> impl Strategy<Value = SystemSdkError> {
    // Variant selector (0..=6) + a message string
    (0u8..7, "[a-zA-Z0-9 _.:/-]{1,80}").prop_map(|(variant, msg)| match variant {
        0 => SystemSdkError::InitFailed(msg),
        1 => SystemSdkError::HealthCheckFailed(msg),
        2 => SystemSdkError::ShutdownFailed(msg),
        3 => SystemSdkError::BridgeError(msg),
        4 => SystemSdkError::from(std::io::Error::new(std::io::ErrorKind::Other, msg)),
        5 => {
            // Produce a serde_json::Error via invalid parse
            let err = serde_json::from_str::<serde_json::Value>("{ bad }}}").unwrap_err();
            SystemSdkError::from(err)
        }
        _ => {
            // Produce a toml::de::Error via invalid parse
            let err = toml::from_str::<toml::Value>("[invalid\nkey = ").unwrap_err();
            SystemSdkError::from(err)
        }
    })
}

proptest! {
    /// **Property 10: Error context helper produces correct subsystem (`Subsystem::Bridge` for System SDK)**
    ///
    /// **Validates: Requirements 9.2, 21.3**
    ///
    /// For any `SystemSdkError` variant, calling `into_error_context()` SHALL
    /// produce an `ErrorContext` with `subsystem == Subsystem::Bridge`, a
    /// non-empty `code`, and a non-empty `message`.
    #[test]
    fn error_context_has_bridge_subsystem_and_nonempty_fields(
        err in arb_system_sdk_error(),
    ) {
        let ctx: ErrorContext = err.into_error_context();

        prop_assert_eq!(
            ctx.subsystem,
            Subsystem::Bridge,
            "SystemSdkError must map to Subsystem::Bridge"
        );
        prop_assert!(
            !ctx.code.is_empty(),
            "ErrorContext code must be non-empty, got {:?}",
            ctx.code
        );
        prop_assert!(
            !ctx.message.is_empty(),
            "ErrorContext message must be non-empty, got {:?}",
            ctx.message
        );
    }

    /// **Property 11: SDK error From conversions preserve original error info**
    ///
    /// **Validates: Requirements 21.2**
    ///
    /// For `std::io::Error`, converting to `SystemSdkError` via `From` SHALL
    /// produce an error whose `Display` output contains the original message.
    #[test]
    fn from_io_error_preserves_info(
        kind_idx in 0u8..6,
        msg in "[a-zA-Z0-9 _]{1,60}",
    ) {
        let kind = match kind_idx {
            0 => std::io::ErrorKind::NotFound,
            1 => std::io::ErrorKind::PermissionDenied,
            2 => std::io::ErrorKind::ConnectionRefused,
            3 => std::io::ErrorKind::TimedOut,
            4 => std::io::ErrorKind::InvalidData,
            _ => std::io::ErrorKind::Other,
        };
        let io_err = std::io::Error::new(kind, msg.clone());
        let sdk_err = SystemSdkError::from(io_err);
        let display = format!("{}", sdk_err);

        prop_assert!(
            display.contains(&msg),
            "Display of SystemSdkError::Io should contain original message {:?}, got {:?}",
            msg,
            display
        );
    }
}

/// **Property 11 (continued): From<serde_json::Error> preserves original error info**
///
/// **Validates: Requirements 21.2**
#[test]
fn from_json_error_preserves_info() {
    let bad_json = "{ not valid }}}";
    let original_err = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
    let original_display = format!("{}", original_err);

    let sdk_err = SystemSdkError::from(original_err);
    let sdk_display = format!("{}", sdk_err);

    assert!(
        sdk_display.contains(&original_display),
        "Display of SystemSdkError::Json should contain original error text.\n\
         Original: {:?}\nSDK error: {:?}",
        original_display,
        sdk_display
    );
}

/// **Property 11 (continued): From<toml::de::Error> preserves original error info**
///
/// **Validates: Requirements 21.2**
#[test]
fn from_toml_error_preserves_info() {
    let bad_toml = "[invalid\nkey = ";
    let original_err = toml::from_str::<toml::Value>(bad_toml).unwrap_err();
    let original_display = format!("{}", original_err);

    let sdk_err = SystemSdkError::from(original_err);
    let sdk_display = format!("{}", sdk_err);

    assert!(
        sdk_display.contains(&original_display),
        "Display of SystemSdkError::Toml should contain original error text.\n\
         Original: {:?}\nSDK error: {:?}",
        original_display,
        sdk_display
    );
}
