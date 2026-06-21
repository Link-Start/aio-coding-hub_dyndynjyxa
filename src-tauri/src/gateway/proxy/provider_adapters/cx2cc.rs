use super::ProviderCapabilities;

pub(crate) fn is_count_tokens_intercept_supported(
    is_claude_count_tokens: bool,
    capabilities: ProviderCapabilities,
) -> bool {
    is_claude_count_tokens && capabilities.supports_count_tokens_local_intercept()
}
