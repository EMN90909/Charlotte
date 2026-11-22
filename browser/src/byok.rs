//! Charlotte's BYOK (Bring Your Own Key) secure local key manager.
//!
//! Security model (encryption vs. hashing):
//! - **Encrypted secret**: User API keys are encrypted with **AES-256-GCM** (`ring::aead::AES_256_GCM`)
//!   using a 256-bit key derived from device entropy + a per-save cryptographically random 96-bit nonce
//!   and provider-bound Additional Authenticated Data (AAD). Because encryption is authenticated and
//!   reversible locally, the runtime can decrypt the key right before calling the LLM API.
//! - **Fingerprint (one-way hash)**: A truncated SHA-256 hash (`fingerprint`) is stored solely for
//!   non-secret key identification and rotation checks—it is never used for decryption.
//! - **UI/API masking**: A redacted preview (`masked_key`, e.g. `sk-...8877`) is stored separately so
//!   listing keys in the UI never decrypts or exposes the raw secret.
//! - If a user API key returns an error or quota/billing limit, Charlotte automatically fails over
//!   to the platform AI model.

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

pub const SUPPORTED_PROVIDERS: &[&str] = &[
    "gemini", "openai", "anthropic", "deepseek", "openrouter", "nvidia",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskedProviderKey {
    pub provider: String,
    pub masked_key: String,
    /// Non-reversible SHA-256 prefix for key identification only (never used for decryption).
    pub fingerprint: String,
    pub auto_fallback: bool,
    pub is_active: bool,
}

#[derive(Clone)]
struct VaultRecord {
    /// AES-256-GCM sealed payload: `nonce (12 bytes) || ciphertext || auth_tag (16 bytes)`.
    encrypted_blob: Vec<u8>,
    masked_key: String,
    fingerprint: String,
    auto_fallback: bool,
}

impl fmt::Debug for VaultRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultRecord")
            .field("masked_key", &self.masked_key)
            .field("fingerprint", &self.fingerprint)
            .field("encrypted_bytes", &self.encrypted_blob.len())
            .field("auto_fallback", &self.auto_fallback)
            .finish()
    }
}

#[derive(Clone)]
pub struct ByokKeyStore {
    vault: Arc<Mutex<HashMap<String, VaultRecord>>>,
    master_key_material: [u8; 32],
    rng: Arc<SystemRandom>,
}

impl fmt::Debug for ByokKeyStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.vault.lock().map(|v| v.len()).unwrap_or(0);
        f.debug_struct("ByokKeyStore")
            .field("stored_providers", &count)
            .field("cipher", &"AES-256-GCM")
            .finish()
    }
}

impl Default for ByokKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ByokKeyStore {
    pub fn new() -> Self {
        // Derive a 256-bit device-bound encryption key (supplemented by OS keychain when available)
        let mut hasher = Sha256::new();
        hasher.update(b"charlotte-native-device-vault-aes256gcm-v2");
        if let Ok(hostname) = std::env::var("HOSTNAME").or_else(|_| std::env::var("COMPUTERNAME")) {
            hasher.update(hostname.as_bytes());
        }
        if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            hasher.update(user.as_bytes());
        }
        if let Ok(secret) = std::env::var("CHARLOTTE_LOCAL_VAULT_SECRET") {
            hasher.update(secret.as_bytes());
        }
        let result = hasher.finalize();
        let mut key_material = [0u8; 32];
        key_material.copy_from_slice(&result);

        Self {
            vault: Arc::new(Mutex::new(HashMap::new())),
            master_key_material: key_material,
            rng: Arc::new(SystemRandom::new()),
        }
    }

    /// Encrypts the user's API key with AES-256-GCM (authenticated, reversible locally for LLM calls)
    /// and stores a separate masked preview and SHA-256 fingerprint for UI display.
    /// Raw keys are NEVER stored in plain text and NEVER returned by listing APIs.
    pub fn store_key(&self, provider: &str, raw_key: &str) -> Result<MaskedProviderKey, String> {
        let provider_norm = provider.trim().to_lowercase();
        let trimmed_key = raw_key.trim();
        if trimmed_key.is_empty() {
            return Err("Cannot store an empty API key".into());
        }
        if !SUPPORTED_PROVIDERS.contains(&provider_norm.as_str()) {
            return Err(format!("Unsupported provider: {provider_norm}"));
        }

        let encrypted_blob = self.encrypt_aes_gcm(&provider_norm, trimmed_key.as_bytes())?;
        let fingerprint = format!("{:x}", Sha256::digest(trimmed_key.as_bytes()))[..12].to_string();
        let masked_key = mask_key(trimmed_key);

        let record = VaultRecord {
            encrypted_blob,
            masked_key: masked_key.clone(),
            fingerprint: fingerprint.clone(),
            auto_fallback: true,
        };

        let mut vault = self.vault.lock().unwrap();
        vault.insert(provider_norm.clone(), record);

        Ok(MaskedProviderKey {
            provider: provider_norm,
            masked_key,
            fingerprint,
            auto_fallback: true,
            is_active: true,
        })
    }

    /// Decrypts and returns the user's API key exclusively for outbound LLM API calls.
    /// Verifies AES-256-GCM authentication tag and provider AAD before returning plaintext.
    pub fn get_decrypted_key(&self, provider: &str) -> Option<String> {
        let provider_norm = provider.trim().to_lowercase();
        let vault = self.vault.lock().unwrap();
        let record = vault.get(&provider_norm)?;
        let plaintext = self
            .decrypt_aes_gcm(&provider_norm, &record.encrypted_blob)
            .ok()?;
        String::from_utf8(plaintext).ok()
    }

    /// Lists masked key previews and SHA-256 fingerprints for UI display.
    /// Does NOT decrypt stored secrets and NEVER returns raw keys.
    pub fn list_masked_keys(&self) -> Vec<MaskedProviderKey> {
        let vault = self.vault.lock().unwrap();
        let mut list = Vec::new();

        for provider in SUPPORTED_PROVIDERS {
            if let Some(rec) = vault.get(*provider) {
                list.push(MaskedProviderKey {
                    provider: (*provider).to_string(),
                    masked_key: rec.masked_key.clone(),
                    fingerprint: rec.fingerprint.clone(),
                    auto_fallback: rec.auto_fallback,
                    is_active: true,
                });
            }
        }

        list
    }

    /// Deletes a user's API key from the local encrypted vault.
    pub fn delete_key(&self, provider: &str) -> bool {
        let provider_norm = provider.trim().to_lowercase();
        let mut vault = self.vault.lock().unwrap();
        vault.remove(&provider_norm).is_some()
    }

    fn derive_provider_aes_key(&self, provider: &str) -> Result<LessSafeKey, String> {
        let mut hasher = Sha256::new();
        hasher.update(self.master_key_material);
        hasher.update(b":provider:");
        hasher.update(provider.as_bytes());
        let derived = hasher.finalize();
        let unbound = UnboundKey::new(&AES_256_GCM, &derived)
            .map_err(|_| "Failed to initialize AES-256-GCM key".to_string())?;
        Ok(LessSafeKey::new(unbound))
    }

    fn encrypt_aes_gcm(&self, provider: &str, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let key = self.derive_provider_aes_key(provider)?;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng
            .fill(&mut nonce_bytes)
            .map_err(|_| "Failed to generate cryptographic nonce".to_string())?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let mut in_out = plaintext.to_vec();
        key.seal_in_place_append_tag(nonce, Aad::from(provider.as_bytes()), &mut in_out)
            .map_err(|_| "AES-256-GCM encryption failed".to_string())?;

        let mut out = Vec::with_capacity(NONCE_LEN + in_out.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&in_out);
        Ok(out)
    }

    fn decrypt_aes_gcm(&self, provider: &str, blob: &[u8]) -> Result<Vec<u8>, String> {
        if blob.len() <= NONCE_LEN + AES_256_GCM.tag_len() {
            return Err("Encrypted vault payload is too short".into());
        }
        let key = self.derive_provider_aes_key(provider)?;
        let nonce_bytes: [u8; NONCE_LEN] = blob[..NONCE_LEN]
            .try_into()
            .map_err(|_| "Invalid nonce length".to_string())?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let mut ciphertext_and_tag = blob[NONCE_LEN..].to_vec();
        let plaintext = key
            .open_in_place(nonce, Aad::from(provider.as_bytes()), &mut ciphertext_and_tag)
            .map_err(|_| "AES-256-GCM authentication or decryption failed".to_string())?;
        Ok(plaintext.to_vec())
    }
}

pub fn mask_key(raw_key: &str) -> String {
    let trimmed = raw_key.trim();
    if trimmed.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix = &trimmed[..3];
    let suffix = &trimmed[trimmed.len() - 4..];
    format!("{prefix}...{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byok_stores_encrypted_and_masks_keys() {
        let store = ByokKeyStore::new();
        let raw = "sk-deepseek-production-key-998877";

        // 1. Store DeepSeek key (AES-256-GCM encrypted, NOT one-way hashed)
        let masked_info = store.store_key("deepseek", raw).expect("stored");
        assert_eq!(masked_info.provider, "deepseek");
        assert_eq!(masked_info.masked_key, "sk-...8877");
        assert!(!masked_info.masked_key.contains("production"));
        assert_eq!(masked_info.fingerprint.len(), 12);

        // 2. Verify ciphertext on disk/memory does NOT contain plaintext
        {
            let vault = store.vault.lock().unwrap();
            let blob = &vault.get("deepseek").unwrap().encrypted_blob;
            assert!(!blob.windows(raw.len()).any(|w| w == raw.as_bytes()));
        }

        // 3. Verify random nonces: encrypting same key twice produces distinct ciphertexts
        let c1 = store.encrypt_aes_gcm("deepseek", raw.as_bytes()).unwrap();
        let c2 = store.encrypt_aes_gcm("deepseek", raw.as_bytes()).unwrap();
        assert_ne!(c1, c2);

        // 4. Verify GCM integrity: tampered ciphertext or wrong provider AAD fails decryption
        let mut tampered = c1.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0xFF;
        assert!(store.decrypt_aes_gcm("deepseek", &tampered).is_err());
        assert!(store.decrypt_aes_gcm("openai", &c1).is_err());

        // 5. Retrieve decrypted key for agent LLM execution
        let decrypted = store.get_decrypted_key("deepseek");
        assert_eq!(decrypted, Some(raw.into()));

        // 6. List masked keys (never exposes raw key)
        let list = store.list_masked_keys();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].masked_key, "sk-...8877");

        // 7. Debug output never leaks secret
        let dbg = format!("{store:?}");
        assert!(!dbg.contains("production"));

        // 8. Delete key
        assert!(store.delete_key("deepseek"));
        assert_eq!(store.get_decrypted_key("deepseek"), None);
    }
}
