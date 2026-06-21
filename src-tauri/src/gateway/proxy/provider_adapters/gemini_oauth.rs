use super::ProviderCapabilities;

pub(crate) fn is_gemini_oauth(capabilities: ProviderCapabilities) -> bool {
    capabilities.gemini_oauth
}
