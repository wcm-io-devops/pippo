use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use std::{env, process};

/// Reads the encryption key either from `PIPPO_CRYPTKEY` environment variable or from the `./.cryptkey` file.
fn provide_secret_key() -> String {
    // Read secret key from PIPPO_CRYPTKEY environment variable; if not provided, use .cryptkey file
    match env::var("PIPPO_CRYPTKEY") {
        Ok(key_from_envvar) => key_from_envvar,
        Err(_) => {
            match std::fs::read_to_string(".cryptkey") {
                Ok(key_from_file) => key_from_file.trim_end().to_string(),
                Err(_) => {
                    eprintln!("❌ PIPPO_CRYPTKEY not set and .cryptkey file not found. Can't do any crypto!");
                    process::exit(1);
                }
            }
        }
    }
}

/// Encrypts a string and returns base64
///
/// # Arguments
///
///  * `input` - The string you want to encrypt
pub fn encrypt(input: &str) -> String {
    let secret_key = provide_secret_key();
    let magic_crypt = new_magic_crypt!(secret_key, 256);
    magic_crypt.encrypt_str_to_base64(input)
}

/// Decrypts a string and returns it
///
/// # Arguments
///
/// * `input` The string you want to decrypt
pub fn decrypt(input: String) -> String {
    let secret_key = provide_secret_key();
    let magic_crypt = new_magic_crypt!(secret_key, 256);
    magic_crypt
        .decrypt_base64_to_string(input)
        .expect("Could not decrypt string - wrong key?")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_workflow() {
        env::set_var("PIPPO_CRYPTKEY", "Test 123@!");
        let test_string = "th!s i$ a 'TEST`";
        let encrypted_value = encrypt(test_string);
        let decrypted_value = decrypt(encrypted_value);

        assert_eq!(test_string, decrypted_value);
    }
}
