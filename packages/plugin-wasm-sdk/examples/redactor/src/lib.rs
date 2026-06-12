use aio_plugin_wasm_sdk::{aio_plugin_entrypoint, HookRequest, HookResult};
use serde_json::json;

fn handle(request: HookRequest) -> HookResult {
    let Some(input) = request
        .context
        .pointer("/request/body/input")
        .and_then(serde_json::Value::as_str)
    else {
        return HookResult::pass();
    };

    if !input.contains("SECRET_") {
        return HookResult::pass();
    }

    HookResult::replace(json!({
        "request": {
            "body": {
                "input": input.replace("SECRET_", "[REDACTED]_")
            }
        }
    }))
}

aio_plugin_entrypoint!(handle);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redactor_example_replaces_secret_marker() {
        let result = handle(HookRequest {
            abi_version: "1.0.0".to_string(),
            plugin_id: "acme.redactor".to_string(),
            hook: "gateway.request.afterBodyRead".to_string(),
            trace_id: None,
            config: json!({}),
            context: json!({ "request": { "body": { "input": "hello SECRET_TOKEN" } } }),
        });

        assert_eq!(
            result.context_patch.unwrap()["request"]["body"]["input"],
            "hello [REDACTED]_TOKEN"
        );
    }
}
