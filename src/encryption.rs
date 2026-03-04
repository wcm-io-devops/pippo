use std::{env, process};

use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce,
};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use rand::RngCore;
use zeroize::Zeroize;

/// Marker for legacy values (magic-crypt)
pub const ENC_MARKER_V1: &str = "$enc";

/// Marker for new values (argon2 + chacha20poly1305)
pub const ENC_MARKER_V2: &str = "$enc2";

/// Reads the encryption key either from `PIPPO_CRYPTKEY` environment variable or from the `./.cryptkey` file.
fn provide_secret_key() -> String {
    match env::var("PIPPO_CRYPTKEY") {
        Ok(key_from_envvar) => key_from_envvar,
        Err(_) => match std::fs::read_to_string(".cryptkey") {
            Ok(key_from_file) => key_from_file.trim_end().to_string(),
            Err(_) => {
                eprintln!("❌ PIPPO_CRYPTKEY not set and .cryptkey file not found. Can't do any crypto!");
                process::exit(1);
            }
        },
    }
}

/// v2 envelope format (base64-encoded):
/// [ version(1) | salt(16) | nonce(12) | ciphertext+tag(..) ]
///
/// - salt: random per encryption, used for Argon2id key derivation (Argon2::default() is Argon2id)
/// - nonce: random per encryption, required by ChaCha20Poly1305 (must never repeat for same key)
/// - ciphertext includes authentication tag (integrity protection)
const VERSION_V2: u8 = 2;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// Encrypts a string and returns base64
///
/// Keeps original behavior for callers that want raw ciphertext (no marker).
#[allow(dead_code)]
pub fn encrypt(input: &str) -> String {
    encrypt_v2_base64(input)
}

/// Encrypts and returns a value prefixed with `$enc2 ` (note the space).
pub fn encrypt_marked(input: &str) -> String {
    format!("{ENC_MARKER_V2} {}", encrypt_v2_base64(input))
}

/// Decrypts a value and returns the plaintext.
///
/// Supports:
/// - `$enc2 <...>` / `$enc2<...>` (new scheme)
/// - `$enc <...>`  / `$enc<...>`  (legacy scheme; also supports "new ciphertext with old marker")
/// - raw base64 (tries new first, then legacy)
pub fn decrypt(input: String) -> String {
    decrypt_auto(input.trim())
        .expect("Could not decrypt string - wrong key or tampered/corrupted ciphertext?")
}

/// Internal: encrypt using v2 scheme -> base64
fn encrypt_v2_base64(input: &str) -> String {
    let password = provide_secret_key();

    // Random salt for key derivation
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    // Derive a 32-byte key from password + salt using Argon2id
    let argon2 = Argon2::default();
    let mut key_bytes = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), &salt, &mut key_bytes)
        .expect("Argon2 key derivation failed");

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));

    // Random nonce (96-bit). Must be unique per key; random is standard here.
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), input.as_bytes())
        .expect("Encryption failed");

    // Build envelope
    let mut blob = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + ciphertext.len());
    blob.push(VERSION_V2);
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);

    // Wipe derived key material
    key_bytes.zeroize();

    B64.encode(blob)
}

/// Internal: decrypt v2 base64 only (expects v2 envelope)
fn decrypt_v2_base64(b64: &str) -> Result<String, String> {
    let blob = B64.decode(b64).map_err(|_| "Invalid base64".to_string())?;

    if blob.len() < 1 + SALT_LEN + NONCE_LEN + 1 {
        return Err("Ciphertext too short".to_string());
    }

    let version = blob[0];
    if version != VERSION_V2 {
        return Err("Not a v2 ciphertext".to_string());
    }

    let salt_start = 1;
    let salt_end = salt_start + SALT_LEN;
    let nonce_start = salt_end;
    let nonce_end = nonce_start + NONCE_LEN;
    let ct_start = nonce_end;

    let salt = &blob[salt_start..salt_end];
    let nonce = &blob[nonce_start..nonce_end];
    let ciphertext = &blob[ct_start..];

    let password = provide_secret_key();

    // Re-derive key
    let argon2 = Argon2::default();
    let mut key_bytes = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key_bytes)
        .map_err(|_| "Argon2 key derivation failed".to_string())?;

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));

    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| "Decryption failed (wrong key or tampered data)".to_string())?;

    key_bytes.zeroize();

    String::from_utf8(plaintext).map_err(|_| "Decrypted text is not valid UTF-8".to_string())
}

/// Internal: decrypt legacy base64 using magic-crypt (v1).
///
/// IMPORTANT:
/// - This is kept ONLY for backwards compatibility so old `$enc...` values keep working.
/// - New encryption must NOT use this.
fn decrypt_legacy_base64(b64: &str) -> Result<String, String> {
    let secret_key = provide_secret_key();
    let magic_crypt = new_magic_crypt!(secret_key, 256);
    magic_crypt
        .decrypt_base64_to_string(b64)
        .map_err(|_| "Legacy decryption failed (wrong key or corrupted data)".to_string())
}

/// Decrypt that understands `$enc` / `$enc2` markers and does new->old fallback.
///
/// Accepts both forms:
/// - "$enc2<base64>" and "$enc2 <base64>"
/// - "$enc<base64>"  and "$enc <base64>"
fn decrypt_auto(input: &str) -> Result<String, String> {
    let s = input.trim();

    // Prefer explicit v2 marker
    if let Some(rest) = s.strip_prefix(ENC_MARKER_V2) {
        let payload = rest.trim_start(); // <-- critical for "$enc2 <b64>"
        return decrypt_v2_base64(payload);
    }

    // Legacy marker: first try v2 (in case people stored new ciphertext but kept $enc),
    // then fall back to legacy magic-crypt.
    if let Some(rest) = s.strip_prefix(ENC_MARKER_V1) {
        let payload = rest.trim_start(); // <-- critical for "$enc <b64>"
        return decrypt_v2_base64(payload).or_else(|_| decrypt_legacy_base64(payload));
    }

    // No marker: try v2, then legacy
    decrypt_v2_base64(s).or_else(|_| decrypt_legacy_base64(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_cryptkey<T>(key: &str, f: impl FnOnce() -> T) -> T {
        // Avoid poisoned-lock panics (tests can run in parallel; a panic poisons the mutex)
        let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());

        let old = env::var("PIPPO_CRYPTKEY").ok();
        env::set_var("PIPPO_CRYPTKEY", key);

        let result = f();

        match old {
            Some(v) => env::set_var("PIPPO_CRYPTKEY", v),
            None => env::remove_var("PIPPO_CRYPTKEY"),
        }

        result
    }

    #[test]
    fn encryption_workflow_v2_raw_base64() {
        with_cryptkey("Test 123@!", || {
            let test_string = "th!s i$ a 'TEST`";
            let encrypted_value = encrypt(test_string);
            let decrypted_value = decrypt(encrypted_value);
            assert_eq!(test_string, decrypted_value);
        });
    }

    #[test]
    fn encryption_workflow_v2_marked_enc2() {
        with_cryptkey("Test 123@!", || {
            let test_string = "hello";
            let encrypted_value = encrypt_marked(test_string);
            let decrypted_value = decrypt(encrypted_value);
            assert_eq!(test_string, decrypted_value);
        });
    }

    #[test]
    fn legacy_enc_marker_still_works() {
        with_cryptkey("Test 123@!", || {
            // Create a legacy ciphertext using magic-crypt (simulates existing stored values).
            let legacy_plain = "legacy-secret";
            let secret_key = provide_secret_key();
            let magic_crypt = new_magic_crypt!(secret_key, 256);
            let legacy_b64 = magic_crypt.encrypt_str_to_base64(legacy_plain);

            // Legacy values commonly appear as "$enc<base64>" (no space)
            let marked_no_space = format!("{ENC_MARKER_V1}{legacy_b64}");
            assert_eq!(legacy_plain, decrypt(marked_no_space));

            // Also accept "$enc <base64>" (with space)
            let marked_with_space = format!("{ENC_MARKER_V1} {legacy_b64}");
            assert_eq!(legacy_plain, decrypt(marked_with_space));
        });
    }

    #[test]
    fn wrong_key_fails() {
        let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());

        let old = env::var("PIPPO_CRYPTKEY").ok();

        env::set_var("PIPPO_CRYPTKEY", "Key-A");
        let encrypted = encrypt_marked("secret"); // v2

        env::set_var("PIPPO_CRYPTKEY", "Key-B");
        assert!(super::decrypt_auto(&encrypted).is_err());

        match old {
            Some(v) => env::set_var("PIPPO_CRYPTKEY", v),
            None => env::remove_var("PIPPO_CRYPTKEY"),
        }
    }

    #[test]
    fn tamper_fails() {
        with_cryptkey("Test 123@!", || {
            let encrypted = encrypt("secret");
            let mut blob = B64.decode(&encrypted).unwrap();

            // Flip one bit in the ciphertext/tag area
            let last = blob.len() - 1;
            blob[last] ^= 1;

            let tampered = B64.encode(blob);
            assert!(decrypt_v2_base64(&tampered).is_err());
        });
    }

    #[test]
    fn nondeterministic_output() {
        with_cryptkey("Test 123@!", || {
            assert_ne!(encrypt("same"), encrypt("same"));
        });
    }
}