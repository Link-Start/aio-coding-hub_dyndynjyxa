//! Usage: Declarative, no-code plugin rule runtime.

use super::privacy_filter::{PrivacyFilter, PrivacyFilterError, PrivacyFilterOptions};
use crate::gateway::plugins::context::{
    GatewayHookAction, GatewayHookResult, GatewayVisibleHookContext,
};
use crate::gateway::plugins::permissions::GatewayPluginError;
use crate::gateway::plugins::pipeline::{GatewayHookFuture, GatewayPluginExecutor};
use crate::plugins::{PluginDetail, PluginRuntime};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub(crate) const MAX_RULE_REGEX_PATTERN_BYTES: usize = 4 * 1024;
const MAX_RULE_REGEX_COMPILED_BYTES: usize = 2 * 1024 * 1024;
const MAX_RULES_PER_RUNTIME: usize = 256;

#[cfg(test)]
static RULE_RUNTIME_TEST_DELAY_MS: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
thread_local! {
    static RULE_RUNTIME_TEST_JSON_PARSE_COUNT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn reset_json_parse_count_for_tests() {
    RULE_RUNTIME_TEST_JSON_PARSE_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn json_parse_count_for_tests() -> u64 {
    RULE_RUNTIME_TEST_JSON_PARSE_COUNT.with(|count| count.get())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleRuntimeError {
    code: &'static str,
    message: String,
}

impl RuleRuntimeError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for RuleRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for RuleRuntimeError {}

#[derive(Debug, Clone)]
pub(crate) struct RuleRuntime {
    rules: Vec<CompiledRule>,
}

impl RuleRuntime {
    #[cfg(test)]
    pub(crate) fn from_value(value: Value) -> Result<Self, RuleRuntimeError> {
        let raw: RuleDocument = serde_json::from_value(value).map_err(|err| {
            RuleRuntimeError::new(
                "PLUGIN_RULE_INVALID_DOCUMENT",
                format!("failed to parse declarative rules: {err}"),
            )
        })?;
        Self::from_document(raw)
    }

    fn from_document(raw: RuleDocument) -> Result<Self, RuleRuntimeError> {
        if raw.rules.len() > MAX_RULES_PER_RUNTIME {
            return Err(RuleRuntimeError::new(
                "PLUGIN_RULE_TOO_MANY_RULES",
                format!("rule document has more than {MAX_RULES_PER_RUNTIME} rules"),
            ));
        }

        let mut rules = Vec::with_capacity(raw.rules.len());
        for rule in raw.rules {
            rules.push(CompiledRule::compile(rule)?);
        }
        Ok(Self { rules })
    }

    pub(crate) fn execute(
        &self,
        context: &GatewayVisibleHookContext,
        config: &Value,
    ) -> Result<GatewayHookResult, RuleRuntimeError> {
        #[cfg(test)]
        if let delay @ 1.. = RULE_RUNTIME_TEST_DELAY_MS.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }

        let mut result = GatewayHookResult::continue_unchanged();
        let mut request_body = context.request.body.clone();
        let mut response_body = context.response.body.clone();
        let mut stream_chunk = context.stream.chunk.clone();
        let mut log_message = context.log.message.clone();

        let rules = self
            .rules
            .iter()
            .filter(|rule| rule.hook == context.hook_name)
            .filter(|rule| rule.when.matches(context, config))
            .collect::<Vec<_>>();
        let mut index = 0usize;
        while index < rules.len() {
            let rule = rules[index];
            let batch_end = json_replace_batch_end(&rules, index);
            let batch = &rules[index..batch_end];
            let matched = if batch.len() > 1 {
                match rule.target.field {
                    TargetField::RequestBody => {
                        apply_json_replace_batch_to_text(&mut request_body, batch)?
                    }
                    TargetField::ResponseBody => {
                        apply_json_replace_batch_to_text(&mut response_body, batch)?
                    }
                    TargetField::StreamChunk => {
                        apply_json_replace_batch_to_text(&mut stream_chunk, batch)?
                    }
                    TargetField::LogMessage => {
                        apply_json_replace_batch_to_text(&mut log_message, batch)?
                    }
                }
            } else {
                match rule.target.field {
                    TargetField::RequestBody => {
                        apply_rule_to_text(&mut request_body, rule, OutputField::RequestBody)?
                    }
                    TargetField::ResponseBody => {
                        apply_rule_to_text(&mut response_body, rule, OutputField::ResponseBody)?
                    }
                    TargetField::StreamChunk => {
                        apply_rule_to_text(&mut stream_chunk, rule, OutputField::StreamChunk)?
                    }
                    TargetField::LogMessage => {
                        apply_rule_to_text(&mut log_message, rule, OutputField::LogMessage)?
                    }
                }
            };

            if !matched {
                index = batch_end;
                continue;
            }

            match &rule.action {
                RuleAction::Replace { .. } => match rule.target.field {
                    TargetField::RequestBody => result.request_body = request_body.clone(),
                    TargetField::ResponseBody => result.response_body = response_body.clone(),
                    TargetField::StreamChunk => result.stream_chunk = stream_chunk.clone(),
                    TargetField::LogMessage => result.log_message = log_message.clone(),
                },
                RuleAction::Block { reason } => {
                    result.action = GatewayHookAction::Block;
                    result.reason = Some(reason.clone());
                    return Ok(result);
                }
                RuleAction::Warn { message } => {
                    result.reason = Some(message.clone());
                }
                RuleAction::AppendMessage { role, content } => {
                    if let Some(next_body) =
                        append_chat_message(request_body.as_deref(), role, content)?
                    {
                        request_body = Some(next_body);
                        result.request_body = request_body.clone();
                    }
                }
            }
            index = batch_end;
        }

        Ok(result)
    }
}

#[derive(Default)]
pub(crate) struct RuleRuntimeGatewayPluginExecutor {
    cache: Mutex<HashMap<String, Arc<RuleRuntime>>>,
    privacy_filter_cache: Mutex<HashMap<String, Arc<PrivacyFilter>>>,
}

impl RuleRuntimeGatewayPluginExecutor {
    pub(crate) fn execute_declarative_rules_plugin(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        let runtime = self.get_or_load_rule_runtime(plugin)?;
        runtime
            .execute(&context, &plugin.config)
            .map_err(to_gateway_plugin_error)
    }

    pub(crate) fn execute_official_privacy_filter_plugin(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        let filter = self.get_or_load_privacy_filter(plugin)?;
        execute_official_privacy_filter_hook(&filter, &context, &plugin.config)
            .map_err(to_privacy_filter_error)
    }

    pub(crate) fn retain_runtime_caches_for_plugins(&self, plugins: &[PluginDetail]) {
        let rule_keys: HashSet<String> = plugins
            .iter()
            .filter(|plugin| {
                matches!(
                    plugin.manifest.runtime,
                    PluginRuntime::DeclarativeRules { .. }
                )
            })
            .map(rule_runtime_cache_key)
            .collect();
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|key, _| rule_keys.contains(key));

        let privacy_keys: HashSet<String> = plugins
            .iter()
            .filter(|plugin| plugin.summary.plugin_id == "official.privacy-filter")
            .map(privacy_filter_cache_key)
            .collect();
        self.privacy_filter_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|key, _| privacy_keys.contains(key));
    }

    fn get_or_load_rule_runtime(
        &self,
        plugin: &PluginDetail,
    ) -> Result<Arc<RuleRuntime>, GatewayPluginError> {
        let cache_key = rule_runtime_cache_key(plugin);
        {
            let cache = self
                .cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(runtime) = cache.get(&cache_key) {
                return Ok(Arc::clone(runtime));
            }
        }

        let runtime = Arc::new(load_rule_runtime(plugin).map_err(to_gateway_plugin_error)?);
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(Arc::clone(
            cache
                .entry(cache_key)
                .or_insert_with(|| Arc::clone(&runtime)),
        ))
    }

    fn get_or_load_privacy_filter(
        &self,
        plugin: &PluginDetail,
    ) -> Result<Arc<PrivacyFilter>, GatewayPluginError> {
        let cache_key = privacy_filter_cache_key(plugin);
        {
            let cache = self
                .privacy_filter_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(filter) = cache.get(&cache_key) {
                return Ok(Arc::clone(filter));
            }
        }

        let filter =
            Arc::new(load_official_privacy_filter(plugin).map_err(to_privacy_filter_error)?);
        let mut cache = self
            .privacy_filter_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(Arc::clone(
            cache
                .entry(cache_key)
                .or_insert_with(|| Arc::clone(&filter)),
        ))
    }
}

fn rule_runtime_cache_key(plugin: &PluginDetail) -> String {
    let version = plugin
        .summary
        .current_version
        .as_deref()
        .unwrap_or(plugin.manifest.version.as_str());
    let installed_dir = plugin.installed_dir.as_deref().unwrap_or("");
    let updated_at = plugin.summary.updated_at;
    let rules = match &plugin.manifest.runtime {
        PluginRuntime::DeclarativeRules { rules } => rules.join("\u{1f}"),
        PluginRuntime::Native { engine } => format!("native:{engine}"),
        PluginRuntime::Wasm { abi_version, .. } => format!("wasm:{abi_version}"),
    };
    format!(
        "{}\u{1e}{}\u{1e}{}\u{1e}{}\u{1e}{}",
        plugin.summary.plugin_id, version, installed_dir, updated_at, rules
    )
}

fn privacy_filter_cache_key(plugin: &PluginDetail) -> String {
    let version = plugin
        .summary
        .current_version
        .as_deref()
        .unwrap_or(plugin.manifest.version.as_str());
    let installed_dir = plugin.installed_dir.as_deref().unwrap_or("");
    let updated_at = plugin.summary.updated_at;
    format!(
        "{}\u{1e}{}\u{1e}{}\u{1e}{}",
        plugin.summary.plugin_id, version, installed_dir, updated_at
    )
}

#[cfg(test)]
impl RuleRuntimeGatewayPluginExecutor {
    fn cache_sizes_for_tests(&self) -> (usize, usize) {
        (
            self.cache.lock().unwrap().len(),
            self.privacy_filter_cache.lock().unwrap().len(),
        )
    }
}

impl GatewayPluginExecutor for RuleRuntimeGatewayPluginExecutor {
    fn execute_request_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        let result = self.execute_declarative_rules_plugin(plugin, context);
        Box::pin(async move { result })
    }

    fn execute_response_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        let result = self.execute_declarative_rules_plugin(plugin, context);
        Box::pin(async move { result })
    }

    fn execute_stream_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        let result = self.execute_declarative_rules_plugin(plugin, context);
        Box::pin(async move { result })
    }

    fn execute_log_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        let result = self.execute_declarative_rules_plugin(plugin, context);
        Box::pin(async move { result })
    }
}

fn load_rule_runtime(plugin: &PluginDetail) -> Result<RuleRuntime, RuleRuntimeError> {
    let PluginRuntime::DeclarativeRules { rules } = &plugin.manifest.runtime else {
        return Err(RuleRuntimeError::new(
            "PLUGIN_RULE_UNSUPPORTED_RUNTIME",
            format!(
                "plugin {} does not use declarativeRules runtime",
                plugin.summary.plugin_id
            ),
        ));
    };
    let root_dir = plugin.installed_dir.as_deref().ok_or_else(|| {
        RuleRuntimeError::new(
            "PLUGIN_RULE_MISSING_INSTALL_DIR",
            format!(
                "plugin {} has no installed_dir for rule loading",
                plugin.summary.plugin_id
            ),
        )
    })?;

    let mut merged_rules = Vec::new();
    for rule_path in rules {
        if rule_path.contains("..") || rule_path.starts_with('/') || rule_path.starts_with('\\') {
            return Err(RuleRuntimeError::new(
                "PLUGIN_RULE_INVALID_PATH",
                format!(
                    "invalid rule path for plugin {}: {rule_path}",
                    plugin.summary.plugin_id
                ),
            ));
        }
        let bytes = fs::read(std::path::Path::new(root_dir).join(rule_path)).map_err(|err| {
            RuleRuntimeError::new(
                "PLUGIN_RULE_READ_FAILED",
                format!(
                    "failed to read rule file for plugin {}: {err}",
                    plugin.summary.plugin_id
                ),
            )
        })?;
        let document: RuleDocument = serde_json::from_slice(&bytes).map_err(|err| {
            RuleRuntimeError::new(
                "PLUGIN_RULE_INVALID_DOCUMENT",
                format!(
                    "failed to parse rule file for plugin {}: {err}",
                    plugin.summary.plugin_id
                ),
            )
        })?;
        merged_rules.extend(document.rules);
    }

    RuleRuntime::from_document(RuleDocument {
        rules: merged_rules,
    })
}

fn load_official_privacy_filter(
    plugin: &PluginDetail,
) -> Result<PrivacyFilter, PrivacyFilterError> {
    let root_dir = plugin.installed_dir.as_deref().ok_or_else(|| {
        PrivacyFilterError::new(format!(
            "plugin {} has no installed_dir for privacy-filter rule loading",
            plugin.summary.plugin_id
        ))
    })?;
    let rules_path = std::path::Path::new(root_dir).join("rules/gitleaks.toml");
    let raw = fs::read_to_string(&rules_path).map_err(|err| {
        PrivacyFilterError::new(format!(
            "failed to read privacy-filter gitleaks rules for plugin {}: {err}",
            plugin.summary.plugin_id
        ))
    })?;
    PrivacyFilter::from_gitleaks_toml(&raw)
}

fn execute_official_privacy_filter_hook(
    filter: &PrivacyFilter,
    context: &GatewayVisibleHookContext,
    config: &Value,
) -> Result<GatewayHookResult, PrivacyFilterError> {
    let mut result = GatewayHookResult::continue_unchanged();
    let options = privacy_filter_options_from_config(config);
    match context.hook_name.as_str() {
        "gateway.request.afterBodyRead" | "gateway.request.beforeSend" => {
            if config.get("redactBeforeUpstream") != Some(&Value::Bool(true)) {
                return Ok(result);
            }
            let Some(body) = context.request.body.as_deref() else {
                return Ok(result);
            };
            if let Some(next_body) = redact_request_body_strings(filter, body, &options)? {
                result.request_body = Some(next_body);
            }
        }
        "log.beforePersist" => {
            if config.get("redactLogs") != Some(&Value::Bool(true)) {
                return Ok(result);
            }
            let Some(message) = context.log.message.as_deref() else {
                return Ok(result);
            };
            let redacted = filter.redact_with_options(message, &options);
            if redacted.hit {
                result.log_message = Some(redacted.redacted);
            }
        }
        _ => {}
    }
    Ok(result)
}

fn privacy_filter_options_from_config(config: &Value) -> PrivacyFilterOptions {
    let sensitive_types = config
        .get("sensitiveTypes")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
    PrivacyFilterOptions::from_sensitive_types(sensitive_types.as_deref())
}

fn redact_request_body_strings(
    filter: &PrivacyFilter,
    body: &str,
    options: &PrivacyFilterOptions,
) -> Result<Option<String>, PrivacyFilterError> {
    let Ok(mut root) = serde_json::from_str::<Value>(body) else {
        let redacted = filter.redact_with_options(body, options);
        return Ok(redacted.hit.then_some(redacted.redacted));
    };
    let mut matched = false;
    redact_json_strings_mut(&mut root, filter, options, &mut matched, None, None);
    if !matched {
        return Ok(None);
    }
    serde_json::to_string(&root)
        .map(Some)
        .map_err(|err| PrivacyFilterError::new(format!("failed to serialize redacted JSON: {err}")))
}

fn redact_json_strings_mut(
    value: &mut Value,
    filter: &PrivacyFilter,
    options: &PrivacyFilterOptions,
    matched: &mut bool,
    key: Option<&str>,
    parent_type: Option<&str>,
) {
    match value {
        Value::String(text) => {
            if is_protocol_identifier_string(key, parent_type) {
                return;
            }
            let redacted = filter.redact_with_options(text, options);
            if redacted.hit {
                *text = redacted.redacted;
                *matched = true;
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_json_strings_mut(item, filter, options, matched, None, None);
            }
        }
        Value::Object(map) => {
            let parent_type = map.get("type").and_then(Value::as_str).map(str::to_string);
            for (key, value) in map.iter_mut() {
                redact_json_strings_mut(
                    value,
                    filter,
                    options,
                    matched,
                    Some(key.as_str()),
                    parent_type.as_deref(),
                );
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn is_protocol_identifier_string(key: Option<&str>, parent_type: Option<&str>) -> bool {
    matches!(
        (parent_type, key),
        (Some("tool_use"), Some("id")) | (Some("tool_result"), Some("tool_use_id"))
    )
}

fn to_privacy_filter_error(err: PrivacyFilterError) -> GatewayPluginError {
    GatewayPluginError::new("PLUGIN_PRIVACY_FILTER_FAILED", err.to_string())
}

fn to_gateway_plugin_error(err: RuleRuntimeError) -> GatewayPluginError {
    GatewayPluginError::new(err.code(), err.to_string())
}

#[derive(Debug, Clone)]
struct CompiledRule {
    id: String,
    hook: String,
    target: RuleTarget,
    regex: Regex,
    action: RuleAction,
    when: RuleWhen,
}

impl CompiledRule {
    fn compile(raw: RawRule) -> Result<Self, RuleRuntimeError> {
        let regex = compile_regex(&raw.id, &raw.matcher.regex, raw.matcher.case_sensitive)?;
        Ok(Self {
            id: raw.id,
            hook: raw.hook,
            target: RuleTarget::compile(raw.target)?,
            regex,
            action: raw.action.validate()?,
            when: raw.when.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct RuleDocument {
    rules: Vec<RawRule>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRule {
    id: String,
    hook: String,
    target: RawRuleTarget,
    #[serde(rename = "match")]
    matcher: RuleMatcher,
    action: RuleAction,
    #[serde(default)]
    when: Option<RuleWhen>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRuleTarget {
    field: String,
    #[serde(default)]
    json_path: Option<String>,
}

#[derive(Debug, Clone)]
struct RuleTarget {
    field: TargetField,
    json_path: Option<Vec<JsonPathSegment>>,
}

impl RuleTarget {
    fn compile(raw: RawRuleTarget) -> Result<Self, RuleRuntimeError> {
        Ok(Self {
            field: TargetField::parse(&raw.field)?,
            json_path: raw.json_path.as_deref().map(parse_json_path).transpose()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetField {
    RequestBody,
    ResponseBody,
    StreamChunk,
    LogMessage,
}

impl TargetField {
    fn parse(value: &str) -> Result<Self, RuleRuntimeError> {
        match value {
            "request.body" => Ok(Self::RequestBody),
            "response.body" => Ok(Self::ResponseBody),
            "stream.chunk" => Ok(Self::StreamChunk),
            "log.message" => Ok(Self::LogMessage),
            _ => Err(RuleRuntimeError::new(
                "PLUGIN_RULE_INVALID_TARGET",
                format!("unsupported rule target field: {value}"),
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuleMatcher {
    regex: String,
    #[serde(default)]
    case_sensitive: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum RuleAction {
    Replace { replacement: String },
    Block { reason: String },
    Warn { message: String },
    AppendMessage { role: String, content: String },
}

impl RuleAction {
    fn validate(self) -> Result<Self, RuleRuntimeError> {
        if let Self::AppendMessage { role, content } = &self {
            if !matches!(role.as_str(), "system" | "developer") {
                return Err(RuleRuntimeError::new(
                    "PLUGIN_RULE_INVALID_ACTION",
                    "appendMessage role must be system or developer",
                ));
            }
            if content.trim().is_empty() {
                return Err(RuleRuntimeError::new(
                    "PLUGIN_RULE_INVALID_ACTION",
                    "appendMessage content must not be empty",
                ));
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuleWhen {
    #[serde(default)]
    cli_keys: Vec<String>,
    #[serde(default)]
    models: Vec<String>,
    #[serde(default)]
    config_equals: std::collections::BTreeMap<String, Value>,
}

impl RuleWhen {
    fn matches(&self, context: &GatewayVisibleHookContext, config: &Value) -> bool {
        if !self.cli_keys.is_empty() {
            let Some(cli_key) = context.request.cli_key.as_deref() else {
                return false;
            };
            if !self.cli_keys.iter().any(|item| item == cli_key) {
                return false;
            }
        }

        if !self.models.is_empty() {
            let Some(model) = context.request.requested_model.as_deref() else {
                return false;
            };
            if !self.models.iter().any(|item| item == model) {
                return false;
            }
        }

        for (key, expected) in &self.config_equals {
            if config.get(key) != Some(expected) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonPathSegment {
    Key(String),
    WildcardArray,
}

#[derive(Debug, Clone, Copy)]
enum OutputField {
    RequestBody,
    ResponseBody,
    StreamChunk,
    LogMessage,
}

fn compile_regex(
    rule_id: &str,
    pattern: &str,
    case_sensitive: Option<bool>,
) -> Result<Regex, RuleRuntimeError> {
    if pattern.len() > MAX_RULE_REGEX_PATTERN_BYTES {
        return Err(RuleRuntimeError::new(
            "PLUGIN_RULE_REGEX_TOO_LARGE",
            format!("regex pattern is too large for rule {rule_id}"),
        ));
    }
    RegexBuilder::new(pattern)
        .case_insensitive(!case_sensitive.unwrap_or(true))
        .size_limit(MAX_RULE_REGEX_COMPILED_BYTES)
        .build()
        .map_err(|err| {
            RuleRuntimeError::new(
                "PLUGIN_RULE_INVALID_REGEX",
                format!("invalid regex for rule {rule_id}: {err}"),
            )
        })
}

fn json_replace_batch_end(rules: &[&CompiledRule], start: usize) -> usize {
    let Some(first) = rules.get(start) else {
        return start;
    };
    let Some(path) = first.target.json_path.as_deref() else {
        return start.saturating_add(1);
    };
    if !matches!(first.action, RuleAction::Replace { .. }) {
        return start.saturating_add(1);
    }

    let mut end = start.saturating_add(1);
    while let Some(rule) = rules.get(end) {
        if rule.target.field != first.target.field
            || rule.target.json_path.as_deref() != Some(path)
            || !matches!(rule.action, RuleAction::Replace { .. })
        {
            break;
        }
        end = end.saturating_add(1);
    }
    end
}

fn apply_json_replace_batch_to_text(
    text: &mut Option<String>,
    rules: &[&CompiledRule],
) -> Result<bool, RuleRuntimeError> {
    let Some(first) = rules.first() else {
        return Ok(false);
    };
    let Some(path) = first.target.json_path.as_deref() else {
        return Ok(false);
    };
    let Some(current) = text.as_mut() else {
        return Ok(false);
    };
    let Some(mut root) = parse_json_or_skip(current)? else {
        return Ok(false);
    };

    let mut matched = false;
    apply_to_json_strings_mut(&mut root, path, &mut |candidate| {
        for rule in rules {
            let RuleAction::Replace { replacement } = &rule.action else {
                continue;
            };
            if rule.regex.is_match(candidate) {
                let next = rule
                    .regex
                    .replace_all(candidate, replacement.as_str())
                    .into_owned();
                *candidate = next;
                matched = true;
            }
        }
    });

    if matched {
        *current = serde_json::to_string(&root).map_err(|err| {
            RuleRuntimeError::new(
                "PLUGIN_RULE_INVALID_OUTPUT",
                format!(
                    "failed to serialize rewritten JSON for rule {}: {err}",
                    first.id
                ),
            )
        })?;
    }

    Ok(matched)
}

fn apply_rule_to_text(
    text: &mut Option<String>,
    rule: &CompiledRule,
    _output_field: OutputField,
) -> Result<bool, RuleRuntimeError> {
    let Some(current) = text.as_mut() else {
        return Ok(false);
    };

    match (&rule.target.json_path, &rule.action) {
        (Some(path), RuleAction::Replace { replacement }) => {
            let Some(mut root) = parse_json_or_skip(current)? else {
                return Ok(false);
            };
            let mut matched = false;
            apply_to_json_strings_mut(&mut root, path, &mut |candidate| {
                if rule.regex.is_match(candidate) {
                    let next = rule
                        .regex
                        .replace_all(candidate, replacement.as_str())
                        .into_owned();
                    *candidate = next;
                    matched = true;
                }
            });
            if matched {
                *current = serde_json::to_string(&root).map_err(|err| {
                    RuleRuntimeError::new(
                        "PLUGIN_RULE_INVALID_OUTPUT",
                        format!(
                            "failed to serialize rewritten JSON for rule {}: {err}",
                            rule.id
                        ),
                    )
                })?;
            }
            Ok(matched)
        }
        (Some(path), _) => {
            let Some(mut root) = parse_json_or_skip(current)? else {
                return Ok(false);
            };
            let mut matched = false;
            apply_to_json_strings_mut(&mut root, path, &mut |candidate| {
                if rule.regex.is_match(candidate) {
                    matched = true;
                }
            });
            Ok(matched)
        }
        (None, RuleAction::Replace { replacement }) => {
            if !rule.regex.is_match(current) {
                return Ok(false);
            }
            *current = rule
                .regex
                .replace_all(current, replacement.as_str())
                .into_owned();
            Ok(true)
        }
        (None, _) => Ok(rule.regex.is_match(current)),
    }
}

fn parse_json_or_skip(text: &str) -> Result<Option<Value>, RuleRuntimeError> {
    #[cfg(test)]
    RULE_RUNTIME_TEST_JSON_PARSE_COUNT.with(|count| {
        count.set(count.get().saturating_add(1));
    });

    match serde_json::from_str::<Value>(text) {
        Ok(value) => Ok(Some(value)),
        Err(err) if err.is_syntax() || err.is_eof() => Ok(None),
        Err(err) => Err(RuleRuntimeError::new(
            "PLUGIN_RULE_INVALID_JSON",
            format!("failed to parse target JSON: {err}"),
        )),
    }
}

fn append_chat_message(
    request_body: Option<&str>,
    role: &str,
    content: &str,
) -> Result<Option<String>, RuleRuntimeError> {
    let Some(request_body) = request_body else {
        return Ok(None);
    };
    let Some(mut root) = parse_json_or_skip(request_body)? else {
        return Ok(None);
    };
    let Some(messages) = root.get_mut("messages").and_then(Value::as_array_mut) else {
        return Ok(None);
    };
    messages.push(serde_json::json!({
        "role": role,
        "content": content,
    }));
    serde_json::to_string(&root).map(Some).map_err(|err| {
        RuleRuntimeError::new(
            "PLUGIN_RULE_INVALID_OUTPUT",
            format!("failed to serialize appended chat message: {err}"),
        )
    })
}

fn apply_to_json_strings_mut<F>(value: &mut Value, path: &[JsonPathSegment], f: &mut F)
where
    F: FnMut(&mut String),
{
    if path.is_empty() {
        if let Value::String(value) = value {
            f(value);
        }
        return;
    }

    match &path[0] {
        JsonPathSegment::Key(key) => {
            if let Some(next) = value.get_mut(key) {
                apply_to_json_strings_mut(next, &path[1..], f);
            }
        }
        JsonPathSegment::WildcardArray => {
            if let Value::Array(items) = value {
                for item in items {
                    apply_to_json_strings_mut(item, &path[1..], f);
                }
            }
        }
    }
}

fn parse_json_path(path: &str) -> Result<Vec<JsonPathSegment>, RuleRuntimeError> {
    let bytes = path.as_bytes();
    if bytes.first() != Some(&b'$') {
        return Err(RuleRuntimeError::new(
            "PLUGIN_RULE_INVALID_JSON_PATH",
            format!("JSON path must start with $: {path}"),
        ));
    }

    let mut segments = Vec::new();
    let mut index = 1usize;
    while index < bytes.len() {
        match bytes[index] {
            b'.' => {
                index += 1;
                let start = index;
                while index < bytes.len() && !matches!(bytes[index], b'.' | b'[') {
                    index += 1;
                }
                if start == index {
                    return Err(RuleRuntimeError::new(
                        "PLUGIN_RULE_INVALID_JSON_PATH",
                        format!("empty JSON path segment: {path}"),
                    ));
                }
                let key = &path[start..index];
                if key.contains('"') || key.contains('\'') {
                    return Err(RuleRuntimeError::new(
                        "PLUGIN_RULE_INVALID_JSON_PATH",
                        format!("quoted JSON path keys are not supported: {path}"),
                    ));
                }
                segments.push(JsonPathSegment::Key(key.to_string()));
            }
            b'[' => {
                if bytes.get(index..index + 3) != Some(b"[*]") {
                    return Err(RuleRuntimeError::new(
                        "PLUGIN_RULE_INVALID_JSON_PATH",
                        format!("only [*] array wildcards are supported: {path}"),
                    ));
                }
                segments.push(JsonPathSegment::WildcardArray);
                index += 3;
            }
            _ => {
                return Err(RuleRuntimeError::new(
                    "PLUGIN_RULE_INVALID_JSON_PATH",
                    format!("unsupported JSON path syntax: {path}"),
                ));
            }
        }
    }

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::plugins::context::{
        GatewayHookAction, GatewayVisibleHookContext, GatewayVisibleLogContext,
        GatewayVisibleRequestContext, GatewayVisibleResponseContext, GatewayVisibleStreamContext,
    };
    use crate::plugins::{
        PluginDetail, PluginHook, PluginHostCompatibility, PluginInstallSource, PluginManifest,
        PluginPermissionRisk, PluginRuntime, PluginStatus, PluginSummary,
    };
    use serde_json::json;
    use std::fs;

    fn context_for_request_body(body: serde_json::Value) -> GatewayVisibleHookContext {
        context_for_request_body_text(body.to_string())
    }

    fn context_for_request_body_text(body: impl Into<String>) -> GatewayVisibleHookContext {
        GatewayVisibleHookContext {
            hook_name: "gateway.request.afterBodyRead".to_string(),
            trace_id: "trace-rule-test".to_string(),
            request: GatewayVisibleRequestContext {
                cli_key: Some("codex".to_string()),
                method: Some("POST".to_string()),
                path: Some("/v1/chat/completions".to_string()),
                query: None,
                headers: None,
                body: Some(body.into()),
                requested_model: Some("gpt-test".to_string()),
                ..GatewayVisibleRequestContext::default()
            },
            response: GatewayVisibleResponseContext::default(),
            stream: GatewayVisibleStreamContext::default(),
            log: GatewayVisibleLogContext::default(),
        }
    }

    fn context_for_log_message(message: &str) -> GatewayVisibleHookContext {
        GatewayVisibleHookContext {
            hook_name: "log.beforePersist".to_string(),
            trace_id: "trace-rule-test".to_string(),
            request: GatewayVisibleRequestContext::default(),
            response: GatewayVisibleResponseContext::default(),
            stream: GatewayVisibleStreamContext::default(),
            log: GatewayVisibleLogContext {
                message: Some(message.to_string()),
            },
        }
    }

    fn context_for_response_body(body: serde_json::Value) -> GatewayVisibleHookContext {
        GatewayVisibleHookContext {
            hook_name: "gateway.response.after".to_string(),
            trace_id: "trace-rule-test".to_string(),
            request: GatewayVisibleRequestContext::default(),
            response: GatewayVisibleResponseContext {
                status: Some(200),
                headers: None,
                body: Some(body.to_string()),
            },
            stream: GatewayVisibleStreamContext::default(),
            log: GatewayVisibleLogContext::default(),
        }
    }

    fn context_for_stream_chunk(chunk: &str) -> GatewayVisibleHookContext {
        GatewayVisibleHookContext {
            hook_name: "gateway.response.chunk".to_string(),
            trace_id: "trace-rule-test".to_string(),
            request: GatewayVisibleRequestContext::default(),
            response: GatewayVisibleResponseContext::default(),
            stream: GatewayVisibleStreamContext {
                sequence: Some(1),
                chunk: Some(chunk.to_string()),
            },
            log: GatewayVisibleLogContext::default(),
        }
    }

    fn rule_plugin(plugin_id: &str, version: &str, installed_dir: String) -> PluginDetail {
        PluginDetail {
            summary: PluginSummary {
                id: 1,
                plugin_id: plugin_id.to_string(),
                name: plugin_id.to_string(),
                current_version: Some(version.to_string()),
                status: PluginStatus::Enabled,
                runtime: "declarativeRules".to_string(),
                permission_risk: PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: PluginManifest {
                id: plugin_id.to_string(),
                name: plugin_id.to_string(),
                version: version.to_string(),
                api_version: "1.0.0".to_string(),
                runtime: PluginRuntime::DeclarativeRules {
                    rules: vec!["rules/main.json".to_string()],
                },
                hooks: vec![PluginHook {
                    name: "gateway.request.afterBodyRead".to_string(),
                    priority: 10,
                    failure_policy: Some("fail-open".to_string()),
                }],
                permissions: vec![
                    "request.body.read".to_string(),
                    "request.body.write".to_string(),
                ],
                host_compatibility: PluginHostCompatibility {
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
            installed_dir: Some(installed_dir),
            config: json!({}),
            granted_permissions: vec![
                "request.body.read".to_string(),
                "request.body.write".to_string(),
            ],
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        }
    }

    fn rule_plugin_with_updated_at(
        plugin_id: &str,
        version: &str,
        installed_dir: String,
        updated_at: i64,
    ) -> PluginDetail {
        let mut plugin = rule_plugin(plugin_id, version, installed_dir);
        plugin.summary.updated_at = updated_at;
        plugin
    }

    fn official_privacy_filter_detail(config: serde_json::Value) -> PluginDetail {
        let fixture = crate::app::plugins::official::official_plugin("official.privacy-filter")
            .expect("official privacy filter fixture");
        let permissions = fixture.manifest.permissions.clone();
        PluginDetail {
            summary: PluginSummary {
                id: 1,
                plugin_id: fixture.manifest.id.clone(),
                name: fixture.manifest.name.clone(),
                current_version: Some(fixture.manifest.version.clone()),
                status: PluginStatus::Enabled,
                runtime: "native:privacyFilter".to_string(),
                permission_risk: PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: fixture.manifest,
            install_source: PluginInstallSource::Official,
            installed_dir: Some(fixture.root_dir.to_string_lossy().to_string()),
            config,
            granted_permissions: permissions,
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        }
    }

    fn write_rule_file(root: &std::path::Path, replacement: &str) {
        let rules_dir = root.join("rules");
        fs::create_dir_all(&rules_dir).expect("create rules dir");
        fs::write(
            rules_dir.join("main.json"),
            json!({
                "rules": [{
                    "id": "replace-secret",
                    "hook": "gateway.request.afterBodyRead",
                    "target": {
                        "field": "request.body",
                        "jsonPath": "$.messages[*].content"
                    },
                    "match": { "regex": "secret" },
                    "action": {
                        "kind": "replace",
                        "replacement": replacement
                    }
                }]
            })
            .to_string(),
        )
        .expect("write rule file");
    }

    #[test]
    fn official_privacy_filter_redacts_phone_numbers_in_provider_request_shapes() {
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let plugin = official_privacy_filter_detail(json!({
            "redactBeforeUpstream": true,
            "redactLogs": true
        }));

        for (name, body) in [
            (
                "claude",
                r#"{"messages":[{"role":"user","content":[{"type":"text","text":"phone 13344441520"}]}]}"#,
            ),
            (
                "openai_chat",
                r#"{"messages":[{"role":"user","content":"phone 13344441520"}]}"#,
            ),
            (
                "codex_responses",
                r#"{"input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"phone 13344441520"}]}]}"#,
            ),
            ("raw_text", "phone 13344441520"),
        ] {
            let context = context_for_request_body_text(body);
            let result = executor
                .execute_official_privacy_filter_plugin(&plugin, context)
                .unwrap_or_else(|err| panic!("{name} privacy filter failed: {err}"));
            let output = result
                .request_body
                .expect("request body should be redacted");
            assert!(
                !output.contains("13344441520"),
                "{name} leaked phone number: {output}"
            );
        }
    }

    #[test]
    fn official_privacy_filter_redacts_before_send_request_bodies() {
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let plugin = official_privacy_filter_detail(json!({
            "redactBeforeUpstream": true,
            "redactLogs": true
        }));

        let mut context = context_for_request_body_text(
            r#"{"input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"phone 13344441520"}]}]}"#,
        );
        context.hook_name = "gateway.request.beforeSend".to_string();

        let result = executor
            .execute_official_privacy_filter_plugin(&plugin, context)
            .expect("privacy filter beforeSend hook");

        let output = result
            .request_body
            .expect("request body should be redacted");
        assert!(output.contains("[电话]"));
        assert!(!output.contains("13344441520"));
    }

    #[test]
    fn official_privacy_filter_preserves_claude_tool_use_protocol_ids() {
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let plugin = official_privacy_filter_detail(json!({
            "redactBeforeUpstream": true,
            "redactLogs": true
        }));
        let tool_use_id = "ghp_abcdefghijklmnopqrstuvwxyzABCDEFGHIJ";
        let context = context_for_request_body_text(
            json!({
                "messages": [
                    {
                        "role": "assistant",
                        "content": [
                            {
                                "type": "tool_use",
                                "id": tool_use_id,
                                "name": "lookup_phone",
                                "input": { "query": "你知道 13344441520 是哪里的手机号嘛" }
                            }
                        ]
                    },
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "tool_result",
                                "tool_use_id": tool_use_id,
                                "content": "手机号 13344441520 查询完成"
                            }
                        ]
                    }
                ]
            })
            .to_string(),
        );

        let result = executor
            .execute_official_privacy_filter_plugin(&plugin, context)
            .expect("privacy filter request hook");

        let output = result
            .request_body
            .expect("request body should be redacted");
        assert!(
            output.contains(tool_use_id),
            "tool id was changed: {output}"
        );
        assert!(output.contains("[电话]"));
        assert!(!output.contains("13344441520"));
    }

    #[test]
    fn official_privacy_filter_redacts_log_messages_after_request_redaction() {
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let plugin = official_privacy_filter_detail(json!({
            "redactBeforeUpstream": true,
            "redactLogs": true
        }));
        let context = context_for_log_message("trace log 13344441520");

        let result = executor
            .execute_official_privacy_filter_plugin(&plugin, context)
            .expect("privacy filter log hook");

        let message = result.log_message.expect("log message should be redacted");
        assert!(!message.contains("13344441520"));
    }

    #[test]
    fn official_privacy_filter_respects_sensitive_types_config() {
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let plugin = official_privacy_filter_detail(json!({
            "redactBeforeUpstream": true,
            "redactLogs": true,
            "sensitiveTypes": ["email"]
        }));

        let result = executor
            .execute_official_privacy_filter_plugin(
                &plugin,
                context_for_request_body_text(
                    r#"{"messages":[{"role":"user","content":"email test@example.com phone 13344441520"}]}"#,
                ),
            )
            .expect("privacy filter request hook");

        let output = result
            .request_body
            .expect("request body should be redacted");
        assert!(output.contains("[邮箱]"));
        assert!(!output.contains("test@example.com"));
        assert!(
            output.contains("13344441520"),
            "cn_phone should remain visible when sensitiveTypes omits it: {output}"
        );
    }

    #[test]
    fn official_privacy_filter_allows_disabling_all_sensitive_types() {
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let plugin = official_privacy_filter_detail(json!({
            "redactBeforeUpstream": true,
            "redactLogs": true,
            "sensitiveTypes": []
        }));

        let result = executor
            .execute_official_privacy_filter_plugin(
                &plugin,
                context_for_request_body_text(
                    r#"{"messages":[{"role":"user","content":"email test@example.com phone 13344441520"}]}"#,
                ),
            )
            .expect("privacy filter request hook");

        assert!(
            result.request_body.is_none(),
            "empty sensitiveTypes should disable every configured strategy"
        );
    }

    #[test]
    fn rule_plugin_runtime_replaces_regex_hits_at_json_path() {
        let runtime = RuleRuntime::from_value(json!({
            "rules": [{
                "id": "redact-api-key",
                "hook": "gateway.request.afterBodyRead",
                "target": {
                    "field": "request.body",
                    "jsonPath": "$.messages[*].content"
                },
                "match": { "regex": "sk-[A-Za-z0-9]{8}" },
                "action": {
                    "kind": "replace",
                    "replacement": "[REDACTED]"
                }
            }]
        }))
        .expect("rules parse");
        let ctx = context_for_request_body(json!({
            "messages": [
                { "role": "user", "content": "token sk-12345678 should disappear" }
            ]
        }));

        let result = runtime.execute(&ctx, &json!({})).expect("rules execute");

        assert_eq!(result.action, GatewayHookAction::Continue);
        let body = result.request_body.expect("rewritten body");
        assert!(body.contains("[REDACTED]"));
        assert!(!body.contains("sk-12345678"));
    }

    #[test]
    fn rule_runtime_batches_same_target_json_rewrites() {
        let runtime = RuleRuntime::from_value(json!({
            "rules": [
                {
                    "id": "redact-api-key",
                    "hook": "gateway.request.afterBodyRead",
                    "target": {
                        "field": "request.body",
                        "jsonPath": "$.messages[*].content"
                    },
                    "match": { "regex": "sk-[A-Za-z0-9]{8}" },
                    "action": {
                        "kind": "replace",
                        "replacement": "[KEY]"
                    }
                },
                {
                    "id": "redact-phone",
                    "hook": "gateway.request.afterBodyRead",
                    "target": {
                        "field": "request.body",
                        "jsonPath": "$.messages[*].content"
                    },
                    "match": { "regex": "1[3-9][0-9]{9}" },
                    "action": {
                        "kind": "replace",
                        "replacement": "[PHONE]"
                    }
                }
            ]
        }))
        .expect("rules parse");
        let ctx = context_for_request_body(json!({
            "messages": [
                {
                    "role": "user",
                    "content": "token sk-12345678 phone 13812345678"
                }
            ]
        }));

        reset_json_parse_count_for_tests();
        let result = runtime.execute(&ctx, &json!({})).expect("rules execute");

        let body = result.request_body.expect("rewritten body");
        assert!(body.contains("[KEY]"));
        assert!(body.contains("[PHONE]"));
        assert!(!body.contains("sk-12345678"));
        assert!(!body.contains("13812345678"));
        assert_eq!(
            json_parse_count_for_tests(),
            1,
            "same target JSON rewrites should parse the body once"
        );
    }

    #[test]
    fn rule_plugin_runtime_blocks_regex_hits_in_response_body() {
        let runtime = RuleRuntime::from_value(json!({
            "rules": [{
                "id": "dangerous-shell",
                "hook": "gateway.response.after",
                "target": {
                    "field": "response.body",
                    "jsonPath": "$.choices[*].message.content"
                },
                "match": { "regex": "rm\\s+-rf\\s+/" },
                "action": {
                    "kind": "block",
                    "reason": "dangerous shell command detected"
                }
            }]
        }))
        .expect("rules parse");
        let ctx = context_for_response_body(json!({
            "choices": [
                { "message": { "content": "Run rm -rf / to clean up." } }
            ]
        }));

        let result = runtime.execute(&ctx, &json!({})).expect("rules execute");

        assert_eq!(result.action, GatewayHookAction::Block);
        assert_eq!(
            result.reason.as_deref(),
            Some("dangerous shell command detected")
        );
    }

    #[test]
    fn rule_plugin_runtime_warns_without_mutating_stream_chunks() {
        let runtime = RuleRuntime::from_value(json!({
            "rules": [{
                "id": "curl-pipe-shell",
                "hook": "gateway.response.chunk",
                "target": { "field": "stream.chunk" },
                "match": { "regex": "curl\\s+[^|]+\\|\\s*sh" },
                "action": {
                    "kind": "warn",
                    "message": "curl pipe shell pattern detected"
                }
            }]
        }))
        .expect("rules parse");
        let ctx = context_for_stream_chunk("data: curl https://example.test/install.sh | sh\n\n");

        let result = runtime.execute(&ctx, &json!({})).expect("rules execute");

        assert_eq!(result.action, GatewayHookAction::Continue);
        assert_eq!(
            result.reason.as_deref(),
            Some("curl pipe shell pattern detected")
        );
        assert_eq!(result.stream_chunk, None);
    }

    #[test]
    fn rule_plugin_runtime_appends_system_or_developer_messages() {
        let runtime = RuleRuntime::from_value(json!({
            "rules": [{
                "id": "append-system-instruction",
                "hook": "gateway.request.afterBodyRead",
                "target": { "field": "request.body" },
                "match": { "regex": "." },
                "action": {
                    "kind": "appendMessage",
                    "role": "system",
                    "content": "Answer concisely."
                }
            }]
        }))
        .expect("rules parse");
        let ctx = context_for_request_body(json!({
            "messages": [
                { "role": "user", "content": "hello" }
            ]
        }));

        let result = runtime.execute(&ctx, &json!({})).expect("rules execute");
        let body: serde_json::Value =
            serde_json::from_str(&result.request_body.expect("rewritten body")).unwrap();
        let messages = body["messages"].as_array().expect("messages array");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1]["role"], "system");
        assert_eq!(messages[1]["content"], "Answer concisely.");
    }

    #[test]
    fn rule_plugin_runtime_rejects_oversized_regex_patterns() {
        let pattern = "a".repeat(MAX_RULE_REGEX_PATTERN_BYTES + 1);
        let err = RuleRuntime::from_value(json!({
            "rules": [{
                "id": "too-large-regex",
                "hook": "log.beforePersist",
                "target": { "field": "log.message" },
                "match": { "regex": pattern },
                "action": { "kind": "warn", "message": "never" }
            }]
        }))
        .unwrap_err();

        assert_eq!(err.code(), "PLUGIN_RULE_REGEX_TOO_LARGE");
    }

    #[test]
    fn rule_runtime_executor_reloads_when_same_plugin_id_version_changes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let v1_dir = dir.path().join("plugin-v1");
        let v2_dir = dir.path().join("plugin-v2");
        write_rule_file(&v1_dir, "[V1]");
        write_rule_file(&v2_dir, "[V2]");
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let ctx = context_for_request_body(json!({
            "messages": [{ "role": "user", "content": "secret" }]
        }));

        let v1 = executor
            .execute_declarative_rules_plugin(
                &rule_plugin(
                    "test.same-plugin",
                    "1.0.0",
                    v1_dir.to_string_lossy().to_string(),
                ),
                ctx.clone(),
            )
            .expect("execute v1");
        assert!(v1.request_body.expect("v1 body").contains("[V1]"));

        let v2 = executor
            .execute_declarative_rules_plugin(
                &rule_plugin(
                    "test.same-plugin",
                    "2.0.0",
                    v2_dir.to_string_lossy().to_string(),
                ),
                ctx,
            )
            .expect("execute v2");

        let body = v2.request_body.expect("v2 body");
        assert!(
            body.contains("[V2]"),
            "expected reloaded v2 rules, got {body}"
        );
        assert!(
            !body.contains("[V1]"),
            "stale v1 rules leaked into v2: {body}"
        );
    }

    #[test]
    fn rule_runtime_executor_reloads_when_same_version_path_updated_at_changes() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_rule_file(dir.path(), "[OLD]");
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let ctx = context_for_request_body(json!({
            "messages": [{ "role": "user", "content": "secret" }]
        }));
        let root = dir.path().to_string_lossy().to_string();

        let old = executor
            .execute_declarative_rules_plugin(
                &rule_plugin_with_updated_at("test.same-plugin", "1.0.0", root.clone(), 1),
                ctx.clone(),
            )
            .expect("execute old rules");
        assert!(old.request_body.expect("old body").contains("[OLD]"));

        write_rule_file(dir.path(), "[NEW]");
        let new = executor
            .execute_declarative_rules_plugin(
                &rule_plugin_with_updated_at("test.same-plugin", "1.0.0", root, 2),
                ctx,
            )
            .expect("execute new rules");

        let body = new.request_body.expect("new body");
        assert!(
            body.contains("[NEW]"),
            "expected updated same-version rules, got {body}"
        );
        assert!(
            !body.contains("[OLD]"),
            "stale same-version rules leaked after updated_at changed: {body}"
        );
    }

    #[test]
    fn rule_runtime_prunes_cache_entries_not_in_active_plugin_keys() {
        let dir = tempfile::tempdir().expect("temp dir");
        let first_dir = dir.path().join("first");
        let second_dir = dir.path().join("second");
        write_rule_file(&first_dir, "[FIRST]");
        write_rule_file(&second_dir, "[SECOND]");
        let executor = RuleRuntimeGatewayPluginExecutor::default();
        let first = rule_plugin(
            "acme.rules",
            "1.0.0",
            first_dir.to_string_lossy().to_string(),
        );
        let second = rule_plugin(
            "acme.other",
            "1.0.0",
            second_dir.to_string_lossy().to_string(),
        );
        let context = context_for_request_body(json!({
            "messages": [{ "role": "user", "content": "secret" }]
        }));

        executor
            .execute_declarative_rules_plugin(&first, context.clone())
            .expect("first rule runtime loads");
        executor
            .execute_declarative_rules_plugin(&second, context)
            .expect("second rule runtime loads");
        assert_eq!(executor.cache_sizes_for_tests().0, 2);

        executor.retain_runtime_caches_for_plugins(&[first]);

        assert_eq!(executor.cache_sizes_for_tests().0, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rule_runtime_cache_does_not_hold_mutex_during_execution() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_rule_file(dir.path(), "[REDACTED]");
        let plugin = Arc::new(rule_plugin(
            "test.slow-plugin",
            "1.0.0",
            dir.path().to_string_lossy().to_string(),
        ));
        let context = Arc::new(context_for_request_body(json!({
            "messages": [{
                "role": "user",
                "content": "aaaaaaaaaaaaaaaaaaaaaaaaa"
            }]
        })));
        let executor = Arc::new(RuleRuntimeGatewayPluginExecutor::default());
        executor
            .execute_declarative_rules_plugin(&plugin, (*context).clone())
            .expect("warm cache");

        RULE_RUNTIME_TEST_DELAY_MS.store(120, Ordering::SeqCst);
        let start = std::time::Instant::now();
        let first_executor = Arc::clone(&executor);
        let first_plugin = Arc::clone(&plugin);
        let first_context = Arc::clone(&context);
        let second_executor = Arc::clone(&executor);
        let second_plugin = Arc::clone(&plugin);
        let second_context = Arc::clone(&context);
        let (first, second) = tokio::join!(
            tokio::task::spawn_blocking(move || {
                first_executor
                    .execute_declarative_rules_plugin(&first_plugin, (*first_context).clone())
            }),
            tokio::task::spawn_blocking(move || {
                second_executor
                    .execute_declarative_rules_plugin(&second_plugin, (*second_context).clone())
            }),
        );

        first.expect("first join").expect("first execution");
        second.expect("second join").expect("second execution");
        RULE_RUNTIME_TEST_DELAY_MS.store(0, Ordering::SeqCst);
        assert!(
            start.elapsed() < std::time::Duration::from_millis(180),
            "runtime executions appear serialized by cache lock: {:?}",
            start.elapsed()
        );
    }
}
