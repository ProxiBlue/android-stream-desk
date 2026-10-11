//! Pairing token: the shared secret a WS client must present in the
//! handshake (`ws://host:port/?token=<token>`). Generated once per install,
//! persisted to `app_config_dir()/pairing.json`, handed out only through
//! channels the machine owner controls (Dashboard QR, `adb` in USB mode).
//!
//! The token is kept in memory for `verify`; it is never written to stdout.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::RwLock;
use subtle::ConstantTimeEq;

const PAIRING_FILE: &str = "pairing.json";
const TOKEN_BYTES: usize = 32;

lazy_static::lazy_static! {
    static ref CURRENT_TOKEN: RwLock<Option<String>> = RwLock::new(None);
}

#[derive(Serialize, Deserialize)]
struct PairingFile {
    token: String,
}

/// 32 bytes from the OS CSPRNG, base64url without padding (43 chars).
pub fn generate_token() -> Result<String, String> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| format!("CSPRNG unavailable: {}", e))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn is_well_formed(token: &str) -> bool {
    URL_SAFE_NO_PAD
        .decode(token)
        .map(|bytes| bytes.len() == TOKEN_BYTES)
        .unwrap_or(false)
}

fn read_token_file(config_dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(config_dir.join(PAIRING_FILE)).ok()?;
    let parsed: PairingFile = serde_json::from_str(&content).ok()?;
    is_well_formed(&parsed.token).then_some(parsed.token)
}

fn write_token_file(config_dir: &Path, token: &str) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| format!("Failed creating directory: {}", e))?;

    let serialized = serde_json::to_string_pretty(&PairingFile {
        token: token.to_string(),
    })
    .map_err(|e| e.to_string())?;
    let final_path = config_dir.join(PAIRING_FILE);
    let tmp_path = config_dir.join(format!("{}.tmp", PAIRING_FILE));

    // Atomic write, same pattern as save_layout_config: a crash mid-write
    // must not leave a truncated token behind.
    std::fs::write(&tmp_path, serialized)
        .map_err(|e| format!("Failed staging pairing file: {}", e))?;
    std::fs::rename(&tmp_path, &final_path)
        .map_err(|e| format!("Failed committing pairing file: {}", e))?;
    Ok(())
}

fn install(token: &str) {
    *CURRENT_TOKEN
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(token.to_string());
}

/// Token currently accepted by the WS server, if one has been loaded.
pub fn current_token() -> Option<String> {
    CURRENT_TOKEN
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Reads the persisted token, or creates and persists a new one when the file
/// is missing or unreadable. The result becomes the token `verify` checks.
pub fn load_or_create(config_dir: &Path) -> Result<String, String> {
    let token = read_or_create_file(config_dir)?;
    install(&token);
    Ok(token)
}

/// Replaces the token with a fresh one. Every device has to pair again.
pub fn rotate(config_dir: &Path) -> Result<String, String> {
    let token = rotate_file(config_dir)?;
    install(&token);
    Ok(token)
}

fn read_or_create_file(config_dir: &Path) -> Result<String, String> {
    if let Some(token) = read_token_file(config_dir) {
        return Ok(token);
    }
    rotate_file(config_dir)
}

fn rotate_file(config_dir: &Path) -> Result<String, String> {
    let token = generate_token()?;
    write_token_file(config_dir, &token)?;
    Ok(token)
}

/// Fallback when `pairing.json` cannot be written: the server still requires
/// a token, it just does not survive a restart.
pub fn install_ephemeral() -> Result<String, String> {
    let token = generate_token()?;
    install(&token);
    Ok(token)
}

/// Constant-time comparison against the current token. An empty candidate or
/// a missing server token never matches.
pub fn verify(candidate: &str) -> bool {
    let current = CURRENT_TOKEN
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match current.as_deref() {
        Some(expected) => tokens_match(expected, candidate),
        None => false,
    }
}

fn tokens_match(expected: &str, candidate: &str) -> bool {
    if expected.is_empty() || candidate.is_empty() {
        return false;
    }
    // `ct_eq` on slices returns 0 early only on a length mismatch; the token
    // length is public (always 43 chars), so that leaks nothing.
    expected.as_bytes().ct_eq(candidate.as_bytes()).into()
}

#[cfg(test)]
pub(crate) const TEST_TOKEN: &str = "dGVzdC10b2tlbi10ZXN0LXRva2VuLXRlc3QtdG9rZW4";

#[cfg(test)]
pub(crate) fn install_for_test(token: &str) {
    install(token);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(test_name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("asd-pairing-{}-{}", test_name, nonce))
    }

    #[test]
    fn generated_tokens_are_32_bytes_and_distinct() {
        let a = generate_token().unwrap();
        let b = generate_token().unwrap();
        assert_eq!(a.len(), 43);
        assert_eq!(URL_SAFE_NO_PAD.decode(&a).unwrap().len(), 32);
        assert!(a
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(a, b);
    }

    #[test]
    fn tokens_match_accepts_only_exact_token() {
        let token = generate_token().unwrap();
        assert!(tokens_match(&token, &token));
        assert!(!tokens_match(&token, ""));
        assert!(!tokens_match("", ""));
        assert!(!tokens_match(&token, &token[..42]));
        let mut flipped = token.clone().into_bytes();
        flipped[0] = if flipped[0] == b'A' { b'B' } else { b'A' };
        assert!(!tokens_match(
            &token,
            std::str::from_utf8(&flipped).unwrap()
        ));
    }

    // Tests that touch the global token all install TEST_TOKEN, so running in
    // parallel with the websocket handshake tests cannot flip their outcome.
    #[test]
    fn verify_checks_the_installed_token() {
        install_for_test(TEST_TOKEN);
        assert!(verify(TEST_TOKEN));
        assert!(!verify(""));
        assert!(!verify("wrong"));
        assert!(!verify(&TEST_TOKEN[..42]));
    }

    // File persistence goes through the non-installing helpers for the same
    // reason.
    #[test]
    fn token_file_is_persisted_and_reused() {
        let dir = unique_temp_dir("load");
        let first = read_or_create_file(&dir).unwrap();
        assert!(dir.join(PAIRING_FILE).exists());
        assert!(!dir.join("pairing.json.tmp").exists());
        let second = read_or_create_file(&dir).unwrap();
        assert_eq!(first, second);

        let rotated = rotate_file(&dir).unwrap();
        assert_ne!(rotated, first);
        assert_eq!(read_or_create_file(&dir).unwrap(), rotated);

        std::fs::write(dir.join(PAIRING_FILE), "{ not json").unwrap();
        let recreated = read_or_create_file(&dir).unwrap();
        assert_ne!(recreated, rotated);
        assert!(is_well_formed(&recreated));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
