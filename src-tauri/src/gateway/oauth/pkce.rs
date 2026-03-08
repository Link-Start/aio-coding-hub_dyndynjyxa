//! Usage: PKCE (Proof Key for Code Exchange) challenge generation.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub(crate) struct PkcePair {
    pub code_verifier: String,
    pub code_challenge: String,
}

pub(crate) fn generate_pkce_pair() -> PkcePair {
    use rand::RngCore;
    let mut buf = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut buf);
    let code_verifier = base64_url_encode(&buf);
    let code_challenge = code_challenge_s256(&code_verifier);
    PkcePair {
        code_verifier,
        code_challenge,
    }
}

pub(crate) fn code_challenge_s256(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    base64_url_encode(&hash)
}

fn base64_url_encode(input: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_pair_generates_valid_lengths() {
        let pair = generate_pkce_pair();
        assert!(!pair.code_verifier.is_empty());
        assert!(!pair.code_challenge.is_empty());
        assert_ne!(pair.code_verifier, pair.code_challenge);
    }

    #[test]
    fn code_challenge_is_deterministic() {
        let a = code_challenge_s256("test_verifier");
        let b = code_challenge_s256("test_verifier");
        assert_eq!(a, b);
    }
}
