use super::ProviderCapabilities;

pub(crate) fn is_anthropic_compatible(capabilities: ProviderCapabilities) -> bool {
    capabilities.anthropic_compatible
}
