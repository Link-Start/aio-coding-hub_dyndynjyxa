use super::ProviderCapabilities;

pub(crate) fn is_codex_chatgpt_backend(capabilities: ProviderCapabilities) -> bool {
    capabilities.codex_chatgpt_backend
}
