//! Usage: Ordered, timeout-bounded gateway plugin hook pipeline.

use super::context::{
    GatewayHookAction, GatewayHookResult, GatewayLogHookInput, GatewayPluginHookName,
    GatewayRequestHookInput, GatewayResponseHookInput, GatewayStreamHookInput,
    GatewayVisibleHookContext,
};
use super::permissions::{enforce_hook_result_permissions, GatewayPluginError};
use crate::domain::plugins::{PluginDetail, PluginStatus};
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

pub(crate) type GatewayHookFuture =
    Pin<Box<dyn Future<Output = Result<GatewayHookResult, GatewayPluginError>> + Send>>;

pub(crate) trait GatewayPluginExecutor: Send + Sync {
    fn execute_request_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture;

    fn execute_response_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture;

    fn execute_stream_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture;

    fn execute_log_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture;
}

struct NoopGatewayPluginExecutor;

impl GatewayPluginExecutor for NoopGatewayPluginExecutor {
    fn execute_request_hook(
        &self,
        _plugin: &PluginDetail,
        _context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) })
    }

    fn execute_response_hook(
        &self,
        _plugin: &PluginDetail,
        _context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) })
    }

    fn execute_stream_hook(
        &self,
        _plugin: &PluginDetail,
        _context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) })
    }

    fn execute_log_hook(
        &self,
        _plugin: &PluginDetail,
        _context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GatewayPluginPipelineConfig {
    pub(crate) hook_timeout: Duration,
    pub(crate) circuit_failure_threshold: u32,
}

impl Default for GatewayPluginPipelineConfig {
    fn default() -> Self {
        Self {
            hook_timeout: Duration::from_millis(150),
            circuit_failure_threshold: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GatewayPluginCircuitSnapshot {
    pub(crate) failure_count: u32,
    pub(crate) open: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayPluginAuditEvent {
    pub(crate) plugin_id: String,
    pub(crate) hook_name: String,
    pub(crate) event_type: String,
    pub(crate) risk_level: String,
    pub(crate) message: String,
    pub(crate) details: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayBlockResponse {
    pub(crate) status: u16,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayRequestHookOutput {
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
    pub(crate) blocked: Option<GatewayBlockResponse>,
    pub(crate) audit_events: Vec<GatewayPluginAuditEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayResponseHookOutput {
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
    pub(crate) blocked: Option<GatewayBlockResponse>,
    pub(crate) audit_events: Vec<GatewayPluginAuditEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayStreamHookOutput {
    pub(crate) chunk: Bytes,
    pub(crate) blocked: Option<GatewayBlockResponse>,
    pub(crate) audit_events: Vec<GatewayPluginAuditEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayLogHookOutput {
    pub(crate) message: String,
    pub(crate) audit_events: Vec<GatewayPluginAuditEvent>,
}

pub(crate) struct GatewayPluginPipeline {
    plugins: RwLock<Arc<Vec<PluginDetail>>>,
    executor: Arc<dyn GatewayPluginExecutor>,
    config: GatewayPluginPipelineConfig,
    circuits: Mutex<HashMap<String, GatewayPluginCircuitSnapshot>>,
}

impl GatewayPluginPipeline {
    pub(crate) fn empty_shared() -> Arc<Self> {
        Arc::new(Self {
            plugins: RwLock::new(Arc::new(Vec::new())),
            executor: Arc::new(NoopGatewayPluginExecutor),
            config: GatewayPluginPipelineConfig::default(),
            circuits: Mutex::new(HashMap::new()),
        })
    }

    #[cfg(test)]
    pub(crate) fn for_tests(
        plugins: Vec<PluginDetail>,
        executor: Arc<dyn GatewayPluginExecutor>,
        config: GatewayPluginPipelineConfig,
    ) -> Self {
        Self {
            plugins: RwLock::new(Arc::new(plugins)),
            executor,
            config,
            circuits: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn for_runtime(
        plugins: Vec<PluginDetail>,
        executor: Arc<dyn GatewayPluginExecutor>,
        config: GatewayPluginPipelineConfig,
    ) -> Self {
        Self {
            plugins: RwLock::new(Arc::new(plugins)),
            executor,
            config,
            circuits: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_tests_shared(
        plugins: Vec<PluginDetail>,
        executor: Arc<dyn GatewayPluginExecutor>,
        config: GatewayPluginPipelineConfig,
    ) -> Arc<Self> {
        Arc::new(Self::for_tests(plugins, executor, config))
    }

    pub(crate) async fn run_request_hook(
        &self,
        input: GatewayRequestHookInput,
    ) -> Result<GatewayRequestHookOutput, GatewayPluginError> {
        let mut headers = input.headers.clone();
        let mut body = input.body.clone();
        let mut audit_events = Vec::new();

        for plugin in self.plugins_for_hook(input.hook_name) {
            if self.circuit_snapshot(&plugin.summary.plugin_id).open {
                audit_events.push(audit_event(
                    &plugin,
                    input.hook_name,
                    "plugin.hook.skipped",
                    "medium",
                    "Plugin hook skipped because its circuit is open",
                    serde_json::json!({ "reason": "circuit_open" }),
                ));
                continue;
            }

            let current_input = GatewayRequestHookInput {
                headers: headers.clone(),
                body: body.clone(),
                ..input.clone()
            };
            let visible = current_input.visible_context(&plugin.granted_permissions);
            let future = self.executor.execute_request_hook(&plugin, visible);
            let result = match tokio::time::timeout(self.config.hook_timeout, future).await {
                Ok(Ok(result)) => result,
                Ok(Err(err)) => {
                    self.record_failure(&plugin.summary.plugin_id);
                    audit_events.push(audit_event(
                        &plugin,
                        input.hook_name,
                        "plugin.hook.failed",
                        "high",
                        "Plugin hook failed",
                        serde_json::json!({ "error": err.to_string() }),
                    ));
                    if failure_policy(&plugin, input.hook_name) == FailurePolicy::FailClosed {
                        return Err(err);
                    }
                    continue;
                }
                Err(_) => {
                    self.record_failure(&plugin.summary.plugin_id);
                    tracing::warn!(
                        plugin_id = %plugin.summary.plugin_id,
                        hook_name = input.hook_name.as_str(),
                        timeout_ms = self.config.hook_timeout.as_millis(),
                        "plugin hook timed out"
                    );
                    audit_events.push(audit_event(
                        &plugin,
                        input.hook_name,
                        "plugin.hook.failed",
                        "high",
                        "Plugin hook timed out",
                        serde_json::json!({ "failureKind": "timeout" }),
                    ));
                    if failure_policy(&plugin, input.hook_name) == FailurePolicy::FailClosed {
                        return Err(GatewayPluginError::new(
                            "PLUGIN_HOOK_TIMEOUT",
                            format!("plugin hook timed out: {}", plugin.summary.plugin_id),
                        ));
                    }
                    continue;
                }
            };

            if let Err(err) = enforce_hook_result_permissions(
                input.hook_name,
                &plugin.granted_permissions,
                &result,
            ) {
                self.record_failure(&plugin.summary.plugin_id);
                audit_events.push(audit_event(
                    &plugin,
                    input.hook_name,
                    "plugin.hook.failed",
                    "high",
                    "Plugin hook returned unauthorized mutations",
                    serde_json::json!({ "error": err.to_string() }),
                ));
                if failure_policy(&plugin, input.hook_name) == FailurePolicy::FailClosed {
                    return Err(err);
                }
                continue;
            }

            self.record_success(&plugin.summary.plugin_id);
            apply_header_patch(&mut headers, &result.headers)?;
            if let Some(next_body) = result.request_body {
                body = Bytes::from(next_body);
            }
            if result.action == GatewayHookAction::Block {
                let reason = result
                    .reason
                    .unwrap_or_else(|| "Plugin blocked gateway request".to_string());
                audit_events.push(audit_event(
                    &plugin,
                    input.hook_name,
                    "plugin.hook.blocked",
                    "high",
                    "Plugin blocked gateway request",
                    serde_json::json!({ "reason": reason }),
                ));
                return Ok(GatewayRequestHookOutput {
                    headers,
                    body,
                    blocked: Some(GatewayBlockResponse {
                        status: 403,
                        reason,
                    }),
                    audit_events,
                });
            }
            audit_events.push(audit_event(
                &plugin,
                input.hook_name,
                "plugin.hook.completed",
                "low",
                "Plugin hook completed",
                serde_json::json!({}),
            ));
        }

        Ok(GatewayRequestHookOutput {
            headers,
            body,
            blocked: None,
            audit_events,
        })
    }

    pub(crate) async fn run_response_hook(
        &self,
        input: GatewayResponseHookInput,
    ) -> Result<GatewayResponseHookOutput, GatewayPluginError> {
        let mut headers = input.headers.clone();
        let mut body = input.body.clone();
        let mut audit_events = Vec::new();

        for plugin in self.plugins_for_hook(input.hook_name) {
            if self.circuit_snapshot(&plugin.summary.plugin_id).open {
                audit_events.push(audit_event(
                    &plugin,
                    input.hook_name,
                    "plugin.hook.skipped",
                    "medium",
                    "Plugin hook skipped because its circuit is open",
                    serde_json::json!({ "reason": "circuit_open" }),
                ));
                continue;
            }

            let current_input = GatewayResponseHookInput {
                headers: headers.clone(),
                body: body.clone(),
                ..input.clone()
            };
            let visible = current_input.visible_context(&plugin.granted_permissions);
            let result = match tokio::time::timeout(
                self.config.hook_timeout,
                self.executor.execute_response_hook(&plugin, visible),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(err)) => {
                    self.record_failure(&plugin.summary.plugin_id);
                    audit_events.push(failed_event(&plugin, input.hook_name, &err.to_string()));
                    if failure_policy(&plugin, input.hook_name) == FailurePolicy::FailClosed {
                        return Err(err);
                    }
                    continue;
                }
                Err(_) => {
                    self.record_failure(&plugin.summary.plugin_id);
                    audit_events.push(timeout_event(&plugin, input.hook_name));
                    if failure_policy(&plugin, input.hook_name) == FailurePolicy::FailClosed {
                        return Err(timeout_error(&plugin.summary.plugin_id));
                    }
                    continue;
                }
            };

            if let Err(err) = enforce_hook_result_permissions(
                input.hook_name,
                &plugin.granted_permissions,
                &result,
            ) {
                self.record_failure(&plugin.summary.plugin_id);
                audit_events.push(failed_event(&plugin, input.hook_name, &err.to_string()));
                if failure_policy(&plugin, input.hook_name) == FailurePolicy::FailClosed {
                    return Err(err);
                }
                continue;
            }

            self.record_success(&plugin.summary.plugin_id);
            apply_header_patch(&mut headers, &result.headers)?;
            if let Some(next_body) = result.response_body {
                body = Bytes::from(next_body);
            }
            if result.action == GatewayHookAction::Block {
                let reason = result
                    .reason
                    .unwrap_or_else(|| "Plugin blocked gateway response".to_string());
                audit_events.push(audit_event(
                    &plugin,
                    input.hook_name,
                    "plugin.hook.blocked",
                    "high",
                    "Plugin blocked gateway response",
                    serde_json::json!({ "reason": reason }),
                ));
                return Ok(GatewayResponseHookOutput {
                    headers,
                    body,
                    blocked: Some(GatewayBlockResponse {
                        status: 502,
                        reason,
                    }),
                    audit_events,
                });
            }
            audit_events.push(completed_event(&plugin, input.hook_name));
        }

        Ok(GatewayResponseHookOutput {
            headers,
            body,
            blocked: None,
            audit_events,
        })
    }

    pub(crate) async fn run_stream_hook(
        &self,
        input: GatewayStreamHookInput,
    ) -> Result<GatewayStreamHookOutput, GatewayPluginError> {
        let hook_name = GatewayPluginHookName::ResponseChunk;
        let mut chunk = input.chunk.clone();
        let mut audit_events = Vec::new();

        for plugin in self.plugins_for_hook(hook_name) {
            if self.circuit_snapshot(&plugin.summary.plugin_id).open {
                audit_events.push(audit_event(
                    &plugin,
                    hook_name,
                    "plugin.hook.skipped",
                    "medium",
                    "Plugin hook skipped because its circuit is open",
                    serde_json::json!({ "reason": "circuit_open" }),
                ));
                continue;
            }

            let current_input = GatewayStreamHookInput {
                chunk: chunk.clone(),
                ..input.clone()
            };
            let visible = current_input.visible_context(&plugin.granted_permissions);
            let result = match tokio::time::timeout(
                self.config.hook_timeout,
                self.executor.execute_stream_hook(&plugin, visible),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(err)) => {
                    self.record_failure(&plugin.summary.plugin_id);
                    audit_events.push(failed_event(&plugin, hook_name, &err.to_string()));
                    if failure_policy(&plugin, hook_name) == FailurePolicy::FailClosed {
                        return Err(err);
                    }
                    continue;
                }
                Err(_) => {
                    self.record_failure(&plugin.summary.plugin_id);
                    audit_events.push(timeout_event(&plugin, hook_name));
                    if failure_policy(&plugin, hook_name) == FailurePolicy::FailClosed {
                        return Err(timeout_error(&plugin.summary.plugin_id));
                    }
                    continue;
                }
            };

            if let Err(err) =
                enforce_hook_result_permissions(hook_name, &plugin.granted_permissions, &result)
            {
                self.record_failure(&plugin.summary.plugin_id);
                audit_events.push(failed_event(&plugin, hook_name, &err.to_string()));
                if failure_policy(&plugin, hook_name) == FailurePolicy::FailClosed {
                    return Err(err);
                }
                continue;
            }

            self.record_success(&plugin.summary.plugin_id);
            if let Some(next_chunk) = result.stream_chunk {
                chunk = Bytes::from(next_chunk);
            }
            if result.action == GatewayHookAction::Block {
                let reason = result
                    .reason
                    .unwrap_or_else(|| "Plugin blocked gateway stream".to_string());
                audit_events.push(audit_event(
                    &plugin,
                    hook_name,
                    "plugin.hook.blocked",
                    "high",
                    "Plugin blocked gateway stream",
                    serde_json::json!({ "reason": reason }),
                ));
                return Ok(GatewayStreamHookOutput {
                    chunk,
                    blocked: Some(GatewayBlockResponse {
                        status: 502,
                        reason,
                    }),
                    audit_events,
                });
            }
            audit_events.push(completed_event(&plugin, hook_name));
        }

        Ok(GatewayStreamHookOutput {
            chunk,
            blocked: None,
            audit_events,
        })
    }

    pub(crate) async fn run_log_hook(
        &self,
        input: GatewayLogHookInput,
    ) -> Result<GatewayLogHookOutput, GatewayPluginError> {
        let hook_name = GatewayPluginHookName::LogBeforePersist;
        let mut message = input.message.clone();
        let mut audit_events = Vec::new();

        for plugin in self.plugins_for_hook(hook_name) {
            if self.circuit_snapshot(&plugin.summary.plugin_id).open {
                audit_events.push(audit_event(
                    &plugin,
                    hook_name,
                    "plugin.hook.skipped",
                    "medium",
                    "Plugin hook skipped because its circuit is open",
                    serde_json::json!({ "reason": "circuit_open" }),
                ));
                continue;
            }

            let current_input = GatewayLogHookInput {
                message: message.clone(),
                ..input.clone()
            };
            let visible = current_input.visible_context(&plugin.granted_permissions);
            let result = match tokio::time::timeout(
                self.config.hook_timeout,
                self.executor.execute_log_hook(&plugin, visible),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(err)) => {
                    self.record_failure(&plugin.summary.plugin_id);
                    audit_events.push(failed_event(&plugin, hook_name, &err.to_string()));
                    if failure_policy(&plugin, hook_name) == FailurePolicy::FailClosed {
                        return Err(err);
                    }
                    continue;
                }
                Err(_) => {
                    self.record_failure(&plugin.summary.plugin_id);
                    audit_events.push(timeout_event(&plugin, hook_name));
                    if failure_policy(&plugin, hook_name) == FailurePolicy::FailClosed {
                        return Err(timeout_error(&plugin.summary.plugin_id));
                    }
                    continue;
                }
            };

            if let Err(err) =
                enforce_hook_result_permissions(hook_name, &plugin.granted_permissions, &result)
            {
                self.record_failure(&plugin.summary.plugin_id);
                audit_events.push(failed_event(&plugin, hook_name, &err.to_string()));
                if failure_policy(&plugin, hook_name) == FailurePolicy::FailClosed {
                    return Err(err);
                }
                continue;
            }

            self.record_success(&plugin.summary.plugin_id);
            if let Some(next_message) = result.log_message {
                message = next_message;
            }
            audit_events.push(completed_event(&plugin, hook_name));
        }

        Ok(GatewayLogHookOutput {
            message,
            audit_events,
        })
    }

    pub(crate) fn circuit_snapshot(&self, plugin_id: &str) -> GatewayPluginCircuitSnapshot {
        self.circuits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(plugin_id)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn replace_plugins(&self, plugins: Vec<PluginDetail>) {
        let active_ids: std::collections::HashSet<String> = plugins
            .iter()
            .map(|plugin| plugin.summary.plugin_id.clone())
            .collect();
        *self
            .plugins
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::new(plugins);
        self.circuits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|plugin_id, _| active_ids.contains(plugin_id));
    }

    fn plugins_for_hook(&self, hook_name: GatewayPluginHookName) -> Vec<PluginDetail> {
        let snapshot = self
            .plugins
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let mut plugins: Vec<PluginDetail> = snapshot
            .iter()
            .filter(|plugin| plugin.summary.status == PluginStatus::Enabled)
            .filter(|plugin| plugin_hook(plugin, hook_name).is_some())
            .cloned()
            .collect();
        plugins.sort_by_key(|plugin| {
            (
                plugin_hook(plugin, hook_name)
                    .map(|hook| hook.priority)
                    .unwrap_or_default(),
                plugin.summary.plugin_id.clone(),
            )
        });
        plugins
    }

    fn record_failure(&self, plugin_id: &str) {
        let mut circuits = self
            .circuits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = circuits.entry(plugin_id.to_string()).or_default();
        entry.failure_count = entry.failure_count.saturating_add(1);
        if entry.failure_count >= self.config.circuit_failure_threshold.max(1) {
            entry.open = true;
        }
    }

    fn record_success(&self, plugin_id: &str) {
        let mut circuits = self
            .circuits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        circuits.insert(
            plugin_id.to_string(),
            GatewayPluginCircuitSnapshot::default(),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailurePolicy {
    FailOpen,
    FailClosed,
}

fn failure_policy(plugin: &PluginDetail, hook_name: GatewayPluginHookName) -> FailurePolicy {
    plugin_hook(plugin, hook_name)
        .and_then(|hook| hook.failure_policy.as_deref())
        .map(|policy| {
            if policy.eq_ignore_ascii_case("fail-closed") {
                FailurePolicy::FailClosed
            } else {
                FailurePolicy::FailOpen
            }
        })
        .unwrap_or(FailurePolicy::FailOpen)
}

fn plugin_hook(
    plugin: &PluginDetail,
    hook_name: GatewayPluginHookName,
) -> Option<&crate::domain::plugins::PluginHook> {
    plugin
        .manifest
        .hooks
        .iter()
        .find(|hook| hook.name == hook_name.as_str())
}

fn audit_event(
    plugin: &PluginDetail,
    hook_name: GatewayPluginHookName,
    event_type: &str,
    risk_level: &str,
    message: &str,
    details: serde_json::Value,
) -> GatewayPluginAuditEvent {
    GatewayPluginAuditEvent {
        plugin_id: plugin.summary.plugin_id.clone(),
        hook_name: hook_name.as_str().to_string(),
        event_type: event_type.to_string(),
        risk_level: risk_level.to_string(),
        message: message.to_string(),
        details,
    }
}

fn completed_event(
    plugin: &PluginDetail,
    hook_name: GatewayPluginHookName,
) -> GatewayPluginAuditEvent {
    audit_event(
        plugin,
        hook_name,
        "plugin.hook.completed",
        "low",
        "Plugin hook completed",
        serde_json::json!({}),
    )
}

fn failed_event(
    plugin: &PluginDetail,
    hook_name: GatewayPluginHookName,
    error: &str,
) -> GatewayPluginAuditEvent {
    audit_event(
        plugin,
        hook_name,
        "plugin.hook.failed",
        "high",
        "Plugin hook failed",
        serde_json::json!({ "error": error }),
    )
}

fn timeout_event(
    plugin: &PluginDetail,
    hook_name: GatewayPluginHookName,
) -> GatewayPluginAuditEvent {
    audit_event(
        plugin,
        hook_name,
        "plugin.hook.failed",
        "high",
        "Plugin hook timed out",
        serde_json::json!({ "failureKind": "timeout" }),
    )
}

fn timeout_error(plugin_id: &str) -> GatewayPluginError {
    GatewayPluginError::new(
        "PLUGIN_HOOK_TIMEOUT",
        format!("plugin hook timed out: {plugin_id}"),
    )
}

fn apply_header_patch(
    headers: &mut HeaderMap,
    patch: &std::collections::BTreeMap<String, String>,
) -> Result<(), GatewayPluginError> {
    for (name, value) in patch {
        if is_reserved_gateway_header(name) {
            return Err(GatewayPluginError::new(
                "PLUGIN_RESERVED_HEADER",
                format!("plugin cannot write reserved gateway header: {name}"),
            ));
        }
        let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|err| {
            GatewayPluginError::new(
                "PLUGIN_INVALID_HEADER",
                format!("invalid header name from plugin result: {err}"),
            )
        })?;
        let header_value = HeaderValue::from_str(value).map_err(|err| {
            GatewayPluginError::new(
                "PLUGIN_INVALID_HEADER",
                format!("invalid header value from plugin result: {err}"),
            )
        })?;
        headers.insert(header_name, header_value);
    }
    Ok(())
}

fn is_reserved_gateway_header(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == "x-trace-id" || lower.starts_with("x-aio-")
}

#[cfg(test)]
type TestRequestHandler = Arc<dyn Fn(GatewayVisibleHookContext) -> GatewayHookFuture + Send + Sync>;

#[cfg(test)]
pub(crate) struct InMemoryGatewayPluginExecutor {
    request_handlers: HashMap<String, TestRequestHandler>,
    response_handlers: HashMap<String, TestRequestHandler>,
    stream_handlers: HashMap<String, TestRequestHandler>,
    log_handlers: HashMap<String, TestRequestHandler>,
}

#[cfg(test)]
impl InMemoryGatewayPluginExecutor {
    pub(crate) fn new() -> Self {
        Self {
            request_handlers: HashMap::new(),
            response_handlers: HashMap::new(),
            stream_handlers: HashMap::new(),
            log_handlers: HashMap::new(),
        }
    }

    pub(crate) fn with_request_handler<F>(mut self, plugin_id: &str, handler: F) -> Self
    where
        F: Fn(GatewayVisibleHookContext) -> GatewayHookResult + Send + Sync + 'static,
    {
        self.request_handlers.insert(
            plugin_id.to_string(),
            Arc::new(move |ctx| {
                let result = handler(ctx);
                Box::pin(async move { Ok(result) })
            }),
        );
        self
    }

    pub(crate) fn with_request_async_handler<F, Fut>(mut self, plugin_id: &str, handler: F) -> Self
    where
        F: Fn(GatewayVisibleHookContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = GatewayHookResult> + Send + 'static,
    {
        self.request_handlers.insert(
            plugin_id.to_string(),
            Arc::new(move |ctx| {
                let future = handler(ctx);
                Box::pin(async move { Ok(future.await) })
            }),
        );
        self
    }

    pub(crate) fn with_response_handler<F>(mut self, plugin_id: &str, handler: F) -> Self
    where
        F: Fn(GatewayVisibleHookContext) -> GatewayHookResult + Send + Sync + 'static,
    {
        self.response_handlers.insert(
            plugin_id.to_string(),
            Arc::new(move |ctx| {
                let result = handler(ctx);
                Box::pin(async move { Ok(result) })
            }),
        );
        self
    }

    pub(crate) fn with_stream_handler<F>(mut self, plugin_id: &str, handler: F) -> Self
    where
        F: Fn(GatewayVisibleHookContext) -> GatewayHookResult + Send + Sync + 'static,
    {
        self.stream_handlers.insert(
            plugin_id.to_string(),
            Arc::new(move |ctx| {
                let result = handler(ctx);
                Box::pin(async move { Ok(result) })
            }),
        );
        self
    }

    pub(crate) fn with_log_handler<F>(mut self, plugin_id: &str, handler: F) -> Self
    where
        F: Fn(GatewayVisibleHookContext) -> GatewayHookResult + Send + Sync + 'static,
    {
        self.log_handlers.insert(
            plugin_id.to_string(),
            Arc::new(move |ctx| {
                let result = handler(ctx);
                Box::pin(async move { Ok(result) })
            }),
        );
        self
    }
}

#[cfg(test)]
impl GatewayPluginExecutor for InMemoryGatewayPluginExecutor {
    fn execute_request_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        match self.request_handlers.get(&plugin.summary.plugin_id) {
            Some(handler) => handler(context),
            None => Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) }),
        }
    }

    fn execute_response_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        match self.response_handlers.get(&plugin.summary.plugin_id) {
            Some(handler) => handler(context),
            None => Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) }),
        }
    }

    fn execute_stream_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        match self.stream_handlers.get(&plugin.summary.plugin_id) {
            Some(handler) => handler(context),
            None => Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) }),
        }
    }

    fn execute_log_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        match self.log_handlers.get(&plugin.summary.plugin_id) {
            Some(handler) => handler(context),
            None => Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plugins::{
        PluginDetail, PluginHook, PluginInstallSource, PluginManifest, PluginRuntime, PluginStatus,
        PluginSummary,
    };
    use axum::body::Bytes;
    use axum::http::{HeaderMap, Method};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    fn plugin(plugin_id: &str, priority: i32, permissions: Vec<&str>) -> PluginDetail {
        PluginDetail {
            summary: PluginSummary {
                id: priority as i64,
                plugin_id: plugin_id.to_string(),
                name: plugin_id.to_string(),
                current_version: Some("1.0.0".to_string()),
                status: PluginStatus::Enabled,
                runtime: "declarativeRules".to_string(),
                permission_risk: crate::domain::plugins::PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: PluginManifest {
                id: plugin_id.to_string(),
                name: plugin_id.to_string(),
                version: "1.0.0".to_string(),
                api_version: "1.0.0".to_string(),
                runtime: PluginRuntime::DeclarativeRules {
                    rules: vec!["rules/main.json".to_string()],
                },
                hooks: vec![PluginHook {
                    name: "gateway.request.afterBodyRead".to_string(),
                    priority,
                    failure_policy: Some("fail-open".to_string()),
                }],
                permissions: permissions.iter().map(|item| item.to_string()).collect(),
                host_compatibility: crate::domain::plugins::PluginHostCompatibility {
                    app: ">=0.56.0 <1.0.0".to_string(),
                    plugin_api: "^1.0.0".to_string(),
                    platforms: vec![],
                },
                entry: None,
                config_schema: None,
                config_version: None,
                description: None,
                author: None,
                homepage: None,
                repository: None,
                license: None,
                checksum: None,
                signature: None,
                category: None,
            },
            install_source: PluginInstallSource::Official,
            installed_dir: None,
            config: serde_json::json!({}),
            granted_permissions: permissions.iter().map(|item| item.to_string()).collect(),
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        }
    }

    fn request_input() -> GatewayRequestHookInput {
        GatewayRequestHookInput {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead,
            trace_id: "trace-ordered".to_string(),
            cli_key: "codex".to_string(),
            method: Method::POST,
            path: "/v1/responses".to_string(),
            query: None,
            headers: HeaderMap::new(),
            body: Bytes::from_static(b"hello"),
            requested_model: None,
        }
    }

    fn response_input() -> GatewayResponseHookInput {
        GatewayResponseHookInput {
            hook_name: GatewayPluginHookName::ResponseAfter,
            trace_id: "trace-response".to_string(),
            status: 200,
            headers: HeaderMap::new(),
            body: Bytes::from_static(b"secret response"),
        }
    }

    fn error_input() -> GatewayResponseHookInput {
        GatewayResponseHookInput {
            hook_name: GatewayPluginHookName::Error,
            trace_id: "trace-error".to_string(),
            status: 502,
            headers: HeaderMap::new(),
            body: Bytes::from_static(
                br#"{"error_code":"GW_UPSTREAM_TIMEOUT","message":"upstream timeout"}"#,
            ),
        }
    }

    fn stream_input() -> GatewayStreamHookInput {
        GatewayStreamHookInput {
            trace_id: "trace-stream".to_string(),
            chunk: Bytes::from_static(b"data: secret\n\n"),
            sequence: 1,
        }
    }

    fn log_input() -> GatewayLogHookInput {
        GatewayLogHookInput {
            trace_id: "trace-log".to_string(),
            message: "token=secret".to_string(),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_pipeline_orders_plugins_and_applies_body_changes() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let calls_a = Arc::clone(&calls);
        let calls_b = Arc::clone(&calls);
        let executor = InMemoryGatewayPluginExecutor::new()
            .with_request_handler("plugin.a", move |ctx| {
                calls_a.lock().unwrap().push("a".to_string());
                assert_eq!(ctx.request.body.as_deref(), Some("hello"));
                GatewayHookResult {
                    request_body: Some("hello a".to_string()),
                    ..GatewayHookResult::continue_unchanged()
                }
            })
            .with_request_handler("plugin.b", move |ctx| {
                calls_b.lock().unwrap().push("b".to_string());
                assert_eq!(ctx.request.body.as_deref(), Some("hello a"));
                GatewayHookResult {
                    request_body: Some("hello a b".to_string()),
                    ..GatewayHookResult::continue_unchanged()
                }
            });
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![
                plugin(
                    "plugin.b",
                    200,
                    vec!["request.body.read", "request.body.write"],
                ),
                plugin(
                    "plugin.a",
                    100,
                    vec!["request.body.read", "request.body.write"],
                ),
            ],
            Arc::new(executor),
            GatewayPluginPipelineConfig {
                hook_timeout: Duration::from_secs(1),
                circuit_failure_threshold: 2,
            },
        );

        let output = pipeline
            .run_request_hook(request_input())
            .await
            .expect("pipeline should succeed");

        assert_eq!(output.body.as_ref(), b"hello a b");
        assert_eq!(calls.lock().unwrap().as_slice(), ["a", "b"]);
        assert!(output.audit_events.iter().any(|event| {
            event.plugin_id == "plugin.a" && event.event_type == "plugin.hook.completed"
        }));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_pipeline_times_out_and_opens_circuit_fail_open() {
        let executor = InMemoryGatewayPluginExecutor::new().with_request_async_handler(
            "plugin.slow",
            |_ctx| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                GatewayHookResult {
                    request_body: Some("late".to_string()),
                    ..GatewayHookResult::continue_unchanged()
                }
            },
        );
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin(
                "plugin.slow",
                10,
                vec!["request.body.read", "request.body.write"],
            )],
            Arc::new(executor),
            GatewayPluginPipelineConfig {
                hook_timeout: Duration::from_millis(1),
                circuit_failure_threshold: 1,
            },
        );

        let first = pipeline
            .run_request_hook(request_input())
            .await
            .expect("fail-open timeout should preserve request");
        assert_eq!(first.body.as_ref(), b"hello");
        assert_eq!(pipeline.circuit_snapshot("plugin.slow").failure_count, 1);
        assert!(pipeline.circuit_snapshot("plugin.slow").open);

        let second = pipeline
            .run_request_hook(request_input())
            .await
            .expect("open circuit should skip plugin");
        assert_eq!(second.body.as_ref(), b"hello");
        assert!(second.audit_events.iter().any(|event| {
            event.plugin_id == "plugin.slow" && event.event_type == "plugin.hook.skipped"
        }));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_pipeline_refreshes_runtime_plugins() {
        let executor = InMemoryGatewayPluginExecutor::new()
            .with_request_handler("plugin.old", |_ctx| GatewayHookResult {
                request_body: Some("old".to_string()),
                ..GatewayHookResult::continue_unchanged()
            })
            .with_request_handler("plugin.new", |_ctx| GatewayHookResult {
                request_body: Some("new".to_string()),
                ..GatewayHookResult::continue_unchanged()
            });
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin(
                "plugin.old",
                10,
                vec!["request.body.read", "request.body.write"],
            )],
            Arc::new(executor),
            GatewayPluginPipelineConfig {
                hook_timeout: Duration::from_secs(1),
                circuit_failure_threshold: 2,
            },
        );

        let before = pipeline
            .run_request_hook(request_input())
            .await
            .expect("pipeline should execute initial plugin");
        assert_eq!(before.body.as_ref(), b"old");

        pipeline.replace_plugins(vec![plugin(
            "plugin.new",
            10,
            vec!["request.body.read", "request.body.write"],
        )]);

        let after = pipeline
            .run_request_hook(request_input())
            .await
            .expect("pipeline should execute refreshed plugin");
        assert_eq!(after.body.as_ref(), b"new");
        assert_eq!(pipeline.circuit_snapshot("plugin.old").failure_count, 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_pipeline_rejects_internal_header_writes() {
        let executor =
            InMemoryGatewayPluginExecutor::new().with_request_handler("plugin.headers", |_ctx| {
                let mut result = GatewayHookResult::continue_unchanged();
                result
                    .headers
                    .insert("x-plugin-safe".to_string(), "ok".to_string());
                result
                    .headers
                    .insert("x-trace-id".to_string(), "spoofed".to_string());
                result
                    .headers
                    .insert("x-aio-provider-id".to_string(), "spoofed".to_string());
                result
                    .headers
                    .insert("x-aio-gateway-forwarded".to_string(), "spoofed".to_string());
                result
            });
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin(
                "plugin.headers",
                10,
                vec!["request.header.read", "request.header.write"],
            )],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let err = pipeline
            .run_request_hook(request_input())
            .await
            .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_RESERVED_HEADER:"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_response_pipeline_applies_body_and_header_changes() {
        let executor =
            InMemoryGatewayPluginExecutor::new().with_response_handler("plugin.response", |ctx| {
                assert_eq!(ctx.response.body.as_deref(), Some("secret response"));
                let mut result = GatewayHookResult {
                    response_body: Some("redacted response".to_string()),
                    ..GatewayHookResult::continue_unchanged()
                };
                result
                    .headers
                    .insert("x-plugin-redacted".to_string(), "1".to_string());
                result
            });
        let mut plugin = plugin(
            "plugin.response",
            10,
            vec![
                "response.header.read",
                "response.header.write",
                "response.body.read",
                "response.body.write",
            ],
        );
        plugin.manifest.hooks[0].name = "gateway.response.after".to_string();

        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let output = pipeline
            .run_response_hook(response_input())
            .await
            .expect("response pipeline should succeed");

        assert_eq!(output.body.as_ref(), b"redacted response");
        assert_eq!(
            output
                .headers
                .get("x-plugin-redacted")
                .and_then(|value| value.to_str().ok()),
            Some("1")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_error_pipeline_applies_error_body_changes() {
        let executor = InMemoryGatewayPluginExecutor::new().with_response_handler(
            "plugin.error",
            |ctx| {
                assert_eq!(ctx.hook_name, "gateway.error");
                assert_eq!(ctx.response.status, Some(502));
                assert!(ctx
                    .response
                    .body
                    .as_deref()
                    .is_some_and(|body| body.contains("GW_UPSTREAM_TIMEOUT")));
                let mut result = GatewayHookResult {
                    response_body: Some(
                        r#"{"error_code":"GW_UPSTREAM_TIMEOUT","message":"redacted upstream error"}"#
                            .to_string(),
                    ),
                    ..GatewayHookResult::continue_unchanged()
                };
                result
                    .headers
                    .insert("x-plugin-error".to_string(), "redacted".to_string());
                result
            },
        );
        let mut plugin = plugin(
            "plugin.error",
            10,
            vec![
                "response.header.read",
                "response.header.write",
                "response.body.read",
                "response.body.write",
            ],
        );
        plugin.manifest.hooks[0].name = "gateway.error".to_string();

        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let output = pipeline
            .run_response_hook(error_input())
            .await
            .expect("error pipeline should succeed");

        assert_eq!(
            std::str::from_utf8(output.body.as_ref()).expect("utf8"),
            r#"{"error_code":"GW_UPSTREAM_TIMEOUT","message":"redacted upstream error"}"#
        );
        assert_eq!(
            output
                .headers
                .get("x-plugin-error")
                .and_then(|value| value.to_str().ok()),
            Some("redacted")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_stream_pipeline_applies_chunk_changes() {
        let executor =
            InMemoryGatewayPluginExecutor::new().with_stream_handler("plugin.stream", |ctx| {
                assert_eq!(ctx.stream.chunk.as_deref(), Some("data: secret\n\n"));
                GatewayHookResult {
                    stream_chunk: Some("data: redacted\n\n".to_string()),
                    ..GatewayHookResult::continue_unchanged()
                }
            });
        let mut plugin = plugin("plugin.stream", 10, vec!["stream.inspect", "stream.modify"]);
        plugin.manifest.hooks[0].name = "gateway.response.chunk".to_string();

        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let output = pipeline
            .run_stream_hook(stream_input())
            .await
            .expect("stream pipeline should succeed");

        assert_eq!(output.chunk.as_ref(), b"data: redacted\n\n");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_log_pipeline_applies_redaction() {
        let executor = InMemoryGatewayPluginExecutor::new().with_log_handler("plugin.log", |ctx| {
            assert_eq!(ctx.log.message.as_deref(), Some("token=secret"));
            GatewayHookResult {
                log_message: Some("token=[REDACTED]".to_string()),
                ..GatewayHookResult::continue_unchanged()
            }
        });
        let mut plugin = plugin("plugin.log", 10, vec!["log.redact"]);
        plugin.manifest.hooks[0].name = "log.beforePersist".to_string();

        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let output = pipeline
            .run_log_hook(log_input())
            .await
            .expect("log pipeline should succeed");

        assert_eq!(output.message, "token=[REDACTED]");
    }
}
