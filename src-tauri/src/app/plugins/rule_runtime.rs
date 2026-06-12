//! Usage: Declarative, no-code plugin rule runtime.

use super::privacy_filter::{PrivacyFilter, PrivacyFilterError};
use crate::gateway::plugins::context::{
    GatewayHookAction, GatewayHookResult, GatewayVisibleHookContext,
};
use crate::gateway::plugins::permissions::GatewayPluginError;
use crate::gateway::plugins::pipeline::{GatewayHookFuture, GatewayPluginExecutor};
use crate::plugins::{PluginDetail, PluginRuntime};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::sync::Mutex;

pub(crate) const MAX_RULE_REGEX_PATTERN_BYTES: usize = 4 * 1024;
const MAX_RULE_REGEX_COMPILED_BYTES: usize = 2 * 1024 * 1024;
const MAX_RULES_PER_RUNTIME: usize = 256;

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
        let mut result = GatewayHookResult::continue_unchanged();
        let mut request_body = context.request.body.clone();
        let mut response_body = context.response.body.clone();
        let mut stream_chunk = context.stream.chunk.clone();
        let mut log_message = context.log.message.clone();

        for rule in self
            .rules
            .iter()
            .filter(|rule| rule.hook == context.hook_name)
        {
            if !rule.when.matches(context, config) {
                continue;
            }

            let matched = match rule.target.field {
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
            };

            if !matched {
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
        }

        Ok(result)
    }
}

#[derive(Default)]
pub(crate) struct RuleRuntimeGatewayPluginExecutor {
    cache: Mutex<HashMap<String, RuleRuntime>>,
    privacy_filter_cache: Mutex<HashMap<String, PrivacyFilter>>,
}

impl RuleRuntimeGatewayPluginExecutor {
    fn execute_plugin(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        if plugin.summary.plugin_id == "official.privacy-filter" {
            return self.execute_official_privacy_filter(plugin, context);
        }

        let cache_key = rule_runtime_cache_key(plugin);
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !cache.contains_key(&cache_key) {
            let runtime = load_rule_runtime(plugin).map_err(to_gateway_plugin_error)?;
            cache.insert(cache_key.clone(), runtime);
        }
        let runtime = cache.get(&cache_key).ok_or_else(|| {
            GatewayPluginError::new(
                "PLUGIN_RULE_RUNTIME_MISSING",
                format!(
                    "rule runtime cache missing for plugin {}",
                    plugin.summary.plugin_id
                ),
            )
        })?;
        runtime
            .execute(&context, &plugin.config)
            .map_err(to_gateway_plugin_error)
    }

    fn execute_official_privacy_filter(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        let cache_key = privacy_filter_cache_key(plugin);
        let mut cache = self
            .privacy_filter_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !cache.contains_key(&cache_key) {
            let filter = load_official_privacy_filter(plugin).map_err(to_privacy_filter_error)?;
            cache.insert(cache_key.clone(), filter);
        }
        let filter = cache.get(&cache_key).ok_or_else(|| {
            GatewayPluginError::new(
                "PLUGIN_PRIVACY_FILTER_RUNTIME_MISSING",
                format!(
                    "privacy filter runtime cache missing for plugin {}",
                    plugin.summary.plugin_id
                ),
            )
        })?;
        execute_official_privacy_filter_hook(filter, &context, &plugin.config)
            .map_err(to_privacy_filter_error)
    }
}

fn rule_runtime_cache_key(plugin: &PluginDetail) -> String {
    let version = plugin
        .summary
        .current_version
        .as_deref()
        .unwrap_or(plugin.manifest.version.as_str());
    let installed_dir = plugin.installed_dir.as_deref().unwrap_or("");
    let rules = match &plugin.manifest.runtime {
        PluginRuntime::DeclarativeRules { rules } => rules.join("\u{1f}"),
        PluginRuntime::Native { engine } => format!("native:{engine}"),
        PluginRuntime::Wasm { abi_version, .. } => format!("wasm:{abi_version}"),
    };
    format!(
        "{}\u{1e}{}\u{1e}{}\u{1e}{}",
        plugin.summary.plugin_id, version, installed_dir, rules
    )
}

fn privacy_filter_cache_key(plugin: &PluginDetail) -> String {
    let version = plugin
        .summary
        .current_version
        .as_deref()
        .unwrap_or(plugin.manifest.version.as_str());
    let installed_dir = plugin.installed_dir.as_deref().unwrap_or("");
    format!(
        "{}\u{1e}{}\u{1e}{}",
        plugin.summary.plugin_id, version, installed_dir
    )
}

impl GatewayPluginExecutor for RuleRuntimeGatewayPluginExecutor {
    fn execute_request_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        let result = self.execute_plugin(plugin, context);
        Box::pin(async move { result })
    }

    fn execute_response_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        let result = self.execute_plugin(plugin, context);
        Box::pin(async move { result })
    }

    fn execute_stream_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        let result = self.execute_plugin(plugin, context);
        Box::pin(async move { result })
    }

    fn execute_log_hook(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
    ) -> GatewayHookFuture {
        let result = self.execute_plugin(plugin, context);
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
    match context.hook_name.as_str() {
        "gateway.request.afterBodyRead" => {
            if config.get("redactBeforeUpstream") != Some(&Value::Bool(true)) {
                return Ok(result);
            }
            let Some(body) = context.request.body.as_deref() else {
                return Ok(result);
            };
            if let Some(next_body) = redact_request_body_strings(filter, body)? {
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
            let redacted = filter.redact(message);
            if redacted.hit {
                result.log_message = Some(redacted.redacted);
            }
        }
        _ => {}
    }
    Ok(result)
}

fn redact_request_body_strings(
    filter: &PrivacyFilter,
    body: &str,
) -> Result<Option<String>, PrivacyFilterError> {
    let Ok(mut root) = serde_json::from_str::<Value>(body) else {
        let redacted = filter.redact(body);
        return Ok(redacted.hit.then_some(redacted.redacted));
    };
    let mut matched = false;
    redact_json_strings_mut(&mut root, filter, &mut matched);
    if !matched {
        return Ok(None);
    }
    serde_json::to_string(&root)
        .map(Some)
        .map_err(|err| PrivacyFilterError::new(format!("failed to serialize redacted JSON: {err}")))
}

fn redact_json_strings_mut(value: &mut Value, filter: &PrivacyFilter, matched: &mut bool) {
    match value {
        Value::String(text) => {
            let redacted = filter.redact(text);
            if redacted.hit {
                *text = redacted.redacted;
                *matched = true;
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_json_strings_mut(item, filter, matched);
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                redact_json_strings_mut(value, filter, matched);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
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
        GatewayVisibleHookContext {
            hook_name: "gateway.request.afterBodyRead".to_string(),
            trace_id: "trace-rule-test".to_string(),
            request: GatewayVisibleRequestContext {
                cli_key: Some("codex".to_string()),
                method: Some("POST".to_string()),
                path: Some("/v1/chat/completions".to_string()),
                query: None,
                headers: None,
                body: Some(body.to_string()),
                requested_model: Some("gpt-test".to_string()),
            },
            response: GatewayVisibleResponseContext::default(),
            stream: GatewayVisibleStreamContext::default(),
            log: GatewayVisibleLogContext::default(),
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
            .execute_plugin(
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
            .execute_plugin(
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
}
