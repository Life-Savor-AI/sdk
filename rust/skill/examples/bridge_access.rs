//! Bridge access example.
//!
//! Demonstrates how a skill accesses system components (cache, file storage,
//! TTS) via the `SystemComponentBridge` protocol using `BridgeRequest`,
//! `BridgeResponse`, `SystemCallRequest`, and `SystemCallResponse`.
//!
//! Run with: `cargo run --example bridge_access`

use lifesavor_skill_sdk::prelude::*;

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Bridge access example — skill → system component calls");

    // --- Cache operations via BridgeRequest ---
    let cache_set = BridgeRequest {
        component: "cache".to_string(),
        operation: "set".to_string(),
        params: serde_json::json!({
            "key": "user:session:123",
            "value": "active",
            "ttl_seconds": 3600
        }),
        skill_id: "my-skill".to_string(),
        correlation_id: Some("corr-001".to_string()),
    };
    info!(
        component = %cache_set.component,
        operation = %cache_set.operation,
        "Cache SET request"
    );

    let cache_get = BridgeRequest {
        component: "cache".to_string(),
        operation: "get".to_string(),
        params: serde_json::json!({ "key": "user:session:123" }),
        skill_id: "my-skill".to_string(),
        correlation_id: Some("corr-001".to_string()),
    };
    info!(
        component = %cache_get.component,
        operation = %cache_get.operation,
        "Cache GET request"
    );

    // --- File storage operations ---
    let file_write = BridgeRequest {
        component: "file_storage".to_string(),
        operation: "write".to_string(),
        params: serde_json::json!({
            "path": "/tmp/skill-output/report.txt",
            "content": "Generated report content"
        }),
        skill_id: "my-skill".to_string(),
        correlation_id: Some("corr-001".to_string()),
    };
    info!(
        component = %file_write.component,
        operation = %file_write.operation,
        "File storage WRITE request"
    );

    // --- TTS synthesis ---
    let tts_request = BridgeRequest {
        component: "tts".to_string(),
        operation: "synthesize".to_string(),
        params: serde_json::json!({
            "text": "Hello from the skill!",
            "voice": "default"
        }),
        skill_id: "my-skill".to_string(),
        correlation_id: Some("corr-001".to_string()),
    };
    info!(
        component = %tts_request.component,
        operation = %tts_request.operation,
        "TTS synthesize request"
    );

    // --- Simulated bridge responses ---
    let ok_response = BridgeResponse::ok(serde_json::json!({ "cached": true }));
    info!(success = ok_response.success, "Cache SET response (OK)");

    let get_response = BridgeResponse::ok(serde_json::json!({
        "value": "active",
        "ttl_remaining": 3540
    }));
    info!(success = get_response.success, "Cache GET response (OK)");

    // Permission denied — skill not allowed to access a component.
    let denied = BridgeResponse::err(
        "BRIDGE_PERMISSION_DENIED",
        "Skill 'my-skill' is not permitted to access 'device_control'",
    );
    info!(
        success = denied.success,
        error = ?denied.error,
        "Permission denied response"
    );

    // --- JSON stdin/stdout protocol envelope ---
    // Skills using JSON stdin/stdout wrap bridge calls in SystemCallRequest.
    let sys_call = SystemCallRequest {
        operation_type: "system_call".to_string(),
        component: "cache".to_string(),
        operation: "delete".to_string(),
        params: serde_json::json!({ "key": "user:session:123" }),
    };
    info!(
        component = %sys_call.component,
        operation = %sys_call.operation,
        "SystemCallRequest (sent over stdin)"
    );

    // Convert BridgeResponse into SystemCallResponse for stdout.
    let bridge_resp = BridgeResponse::ok(serde_json::json!({ "deleted": true }));
    let sys_resp: SystemCallResponse = bridge_resp.into();
    info!(
        success = sys_resp.success,
        operation_type = %sys_resp.operation_type,
        "SystemCallResponse (written to stdout)"
    );

    // --- Serialization round-trip ---
    let request_json = serde_json::to_string_pretty(&cache_set)
        .expect("BridgeRequest serializes");
    info!("BridgeRequest JSON:\n{request_json}");

    let response_json = serde_json::to_string_pretty(&ok_response)
        .expect("BridgeResponse serializes");
    info!("BridgeResponse JSON:\n{response_json}");

    info!("Bridge access example complete");
}
