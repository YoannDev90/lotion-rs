use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
    Key,
    Nonce, // Or `Nonce` from `aes_gcm::aead::AeadCore`
};
use base64::{engine::general_purpose, Engine as _};
use pbkdf2::pbkdf2_hmac;
use rand::Rng;
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::path::PathBuf;

// const APPLICATION_SECRET: &[u8] = b"lotion-rs-super-secret-key-that-is-long-and-random-for-pbkdf2";
const PBKDF2_ITERATIONS: u32 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedState {
    pub data: String,  // Base64 encoded encrypted data
    pub nonce: String, // Base64 encoded nonce
}

fn get_encryption_key(app_secret: &[u8]) -> Key<Aes256Gcm> {
    // Use a stable, but unique per-machine, salt for PBKDF2.
    // Combining the application name and config directory path provides this.
    let salt_source = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lotion-rs")
        .to_string_lossy()
        .into_owned();

    let mut key_bytes = Key::<Aes256Gcm>::default();
    pbkdf2_hmac::<Sha256>(
        app_secret,
        salt_source.as_bytes(),
        PBKDF2_ITERATIONS,
        &mut key_bytes,
    );
    key_bytes
}

fn encrypt_data(data: &[u8], key: &Key<Aes256Gcm>) -> Result<(Vec<u8>, Vec<u8>), String> {
    let cipher = Aes256Gcm::new(key);
    let mut rng = rand::rng();
    let mut nonce_bytes = vec![0u8; 12]; // GCM nonces are 12 bytes
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .encrypt(nonce, data)
        .map(|cipher_text| (cipher_text, nonce_bytes))
        .map_err(|e| format!("Encryption error: {:?}", e))
}

fn decrypt_data(
    encrypted_data: &[u8],
    nonce_bytes: &[u8],
    key: &Key<Aes256Gcm>,
) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, encrypted_data)
        .map_err(|e| format!("Decryption error: {:?}", e))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    pub id: String,
    pub title: String,
    pub url: String,
    pub is_active: bool,
    pub is_pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub id: String,
    pub bounds: Bounds,
    pub is_focused: bool,
    pub is_maximized: bool,
    pub is_minimized: bool,
    pub is_full_screen: bool,
    pub tab_ids: Vec<String>,
    pub active_tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub windows: HashMap<String, WindowState>,
    pub tabs: HashMap<String, TabState>,
    pub focused_window_id: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            tabs: HashMap::new(),
            focused_window_id: None,
        }
    }

    /// Returns the state file path (~/.config/lotion-rs/state.json)
    fn state_path() -> std::path::PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("lotion-rs")
            .join("state.json")
    }

    /// Save application state to disk.
    pub fn save_to_disk(&self, app_secret: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::state_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let key = get_encryption_key(app_secret);
        let plaintext = serde_json::to_string(self)?;
        let (ciphertext, nonce) = encrypt_data(plaintext.as_bytes(), &key)
            .map_err(|e| Box::<dyn std::error::Error>::from(e))?;

        let encrypted_state = EncryptedState {
            data: general_purpose::STANDARD.encode(&ciphertext),
            nonce: general_purpose::STANDARD.encode(&nonce),
        };
        let json = serde_json::to_string_pretty(&encrypted_state)?;

        std::fs::write(&path, json)?;
        log::info!("Encrypted AppState saved to {}", path.display());
        Ok(())
    }

    /// Load application state from disk.
    pub fn load_from_disk(app_secret: &[u8]) -> Option<Self> {
        let path = Self::state_path();
        if path.exists() {
            let key = get_encryption_key(app_secret);
            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    // Try to load as encrypted state first
                    if let Ok(encrypted_state) = serde_json::from_str::<EncryptedState>(&contents) {
                        let decoded_data =
                            match general_purpose::STANDARD.decode(&encrypted_state.data) {
                                Ok(d) => d,
                                Err(e) => {
                                    log::warn!("Failed to base64 decode encrypted data: {}", e);
                                    return None;
                                }
                            };
                        let decoded_nonce =
                            match general_purpose::STANDARD.decode(&encrypted_state.nonce) {
                                Ok(n) => n,
                                Err(e) => {
                                    log::warn!("Failed to base64 decode nonce: {}", e);
                                    return None;
                                }
                            };

                        match decrypt_data(&decoded_data, &decoded_nonce, &key) {
                            Ok(plaintext_bytes) => match String::from_utf8(plaintext_bytes) {
                                Ok(plaintext) => match serde_json::from_str::<AppState>(&plaintext)
                                {
                                    Ok(state) => {
                                        log::info!(
                                            "Encrypted AppState loaded from {}",
                                            path.display()
                                        );
                                        return Some(state);
                                    }
                                    Err(e) => log::warn!("Failed to parse decrypted state: {}", e),
                                },
                                Err(e) => {
                                    log::warn!("Failed to convert decrypted bytes to string: {}", e)
                                }
                            },
                            Err(e) => log::warn!("Failed to decrypt state file: {}", e),
                        }
                    }

                    // If encrypted load failed, try to load as plaintext (for backward compatibility)
                    // This branch will be executed if deserialization to EncryptedState fails,
                    // which means it's likely an old unencrypted file.
                    if let Ok(state) = serde_json::from_str::<AppState>(&contents) {
                        log::warn!(
                            "Loaded unencrypted AppState from {}. Please re-save to encrypt.",
                            path.display()
                        );
                        return Some(state);
                    }
                    log::warn!("Failed to load state file as either encrypted or plaintext.");
                }
                Err(e) => log::warn!("Failed to read state file: {}", e),
            }
        }
        None
    }
}
