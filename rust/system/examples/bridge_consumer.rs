//! Bridge consumer example.
//!
//! Demonstrates how a sandboxed skill accesses system components via the
//! `SystemComponentBridge` protocol. Shows constructing `BridgeRequest`s,
//! handling `BridgeResponse`s, and using the JSON stdin/stdout
//! `SystemCallRequest`/`SystemCallResponse` envelope.
//!
//! Run with: `cargo run --example bridge_consumer`

use lifesavor_system_sdk::prelude::*;

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Bridge consumer example — demonstrating skill → system component calls");

    // --- Construct a BridgeRequest for a cache SET operation ---
    let set_request = BridgeRequest {
        component: "cache".to_string(),
        operation: "set".to_string(),
        params: serde_json::json!({
            "key": "session:abc",
            "value": "active"
        }),
        skill_id: "my-skill".to_string(),
        correlation_id: Some("corr-bridge-001".to_string()),
    };

    info!(
        component = %set_request.component,
        operation = %set_request.operation,
        skill_id = %set_request.skill_id,
        "Constructed cache SET BridgeRequest"
    );

    // --- Construct a BridgeRequest for a cache GET operation ---
    let get_request = BridgeRequest {
        component: "cache".to_string(),
        operation: "get".to_string(),
        params: serde_json::json!({ "key": "session:abc" }),
        skill_id: "my-skill".to_string(),
        correlation_id: Some("corr-bridge-001".to_string()),
    };

    info!(
        component = %get_request.component,
        operation = %get_request.operation,
        "Constructed cache GET BridgeRequest"
    );

    // --- Construct a BridgeRequest for TTS synthesis ---
    let tts_request = BridgeRequest {
        component: "tts".to_string(),
        operation: "synthesize".to_string(),
        params: serde_json::json!({ "text": "Hello from the bridge!" }),
        skill_id: "my-skill".to_string(),
        correlation_id: Some("corr-bridge-001".to_string()),
    };

    info!(
        component = %tts_request.component,
        operation = %tts_request.operation,
        "Constructed TTS BridgeRequest"
    );

    // --- Simulate bridge responses ---
    // In production the SystemComponentBridge dispatches these requests
    // and enforces permissions, rate limits, and PII interception.

    let ok_response = BridgeResponse::ok(serde_json::json!({ "cached": true }));
    info!(success = ok_response.success, "Simulated OK response");

    let err_response = BridgeResponse::err(
        "BRIDGE_PERMISSION_DENIED",
        "Skill 'my-skill' is not permitted to access 'file_storage'",
    );
    info!(
        success = err_response.success,
        error = ?err_response.error,
        "Simulated permission-denied response"
    );

    // --- JSON stdin/stdout protocol envelope ---
    // Skills using JSON stdin/stdout wrap bridge calls in SystemCallRequest.
    let sys_call = SystemCallRequest {
        operation_type: "system_call".to_string(),
        component: "cache".to_string(),
        operation: "delete".to_string(),
        params: serde_json::json!({ "key": "session:abc" }),
    };

    info!(
        component = %sys_call.component,
        operation = %sys_call.operation,
        "SystemCallRequest constructed (sent over stdin in production)"
    );

    // Convert a BridgeResponse into a SystemCallResponse for stdout.
    let bridge_resp = BridgeResponse::ok(serde_json::json!({ "deleted": true }));
    let sys_resp: SystemCallResponse = bridge_resp.into();
    info!(
        success = sys_resp.success,
        operation_type = %sys_resp.operation_type,
        "SystemCallResponse (written to stdout in production)"
    );

    // --- Demonstrate serialization round-trip ---
    let request_json = serde_json::to_string_pretty(&set_request)
        .expect("BridgeRequest should serialize");
    info!("BridgeRequest JSON:\n{}", request_json);

    let response_json = serde_json::to_string_pretty(&ok_response)
        .expect("BridgeResponse should serialize");
    info!("BridgeResponse JSON:\n{}", response_json);

    info!("Bridge consumer example complete");
}
