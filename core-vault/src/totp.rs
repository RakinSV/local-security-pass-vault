use serde::Serialize;
use totp_rs::{Algorithm, Secret, TOTP};

#[derive(Debug, Serialize)]
pub struct TotpCode {
    pub code: String,
    pub valid_for_secs: u32,
    pub period_secs: u32,
}

/// Generates the current TOTP code from a Base32-encoded secret (RFC 6238, SHA-1, 6 digits, 30 s).
/// Accepts raw Base32 secrets (what sites show as "manual entry" key) or the `secret=` value
/// extracted from an `otpauth://` URI.
pub fn generate(secret_base32: &str) -> Result<TotpCode, String> {
    let secret = Secret::Encoded(secret_base32.trim().to_uppercase())
        .to_bytes()
        .map_err(|e| format!("invalid TOTP secret: {e}"))?;
    let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret)
        .map_err(|e| format!("TOTP init error: {e}"))?;
    let code = totp.generate_current().map_err(|e| e.to_string())?;
    let ttl = totp.ttl().map_err(|e| e.to_string())? as u32;
    Ok(TotpCode { code, valid_for_secs: ttl, period_secs: 30 })
}

/// Parses an `otpauth://totp/...` URI and extracts the secret, issuer, and account label.
/// Returns `(secret_base32, issuer, account_name)`.
pub fn parse_otpauth_uri(uri: &str) -> Result<(String, String, String), String> {
    let uri = uri.trim();
    let rest = uri
        .strip_prefix("otpauth://totp/")
        .ok_or("URI must start with otpauth://totp/")?;

    let (label_enc, query) = if let Some(idx) = rest.find('?') {
        (&rest[..idx], &rest[idx + 1..])
    } else {
        (rest, "")
    };

    let label = percent_decode(label_enc);
    let mut secret = String::new();
    let mut issuer = String::new();
    let mut account = label.clone();

    for part in query.split('&') {
        if let Some(v) = part.strip_prefix("secret=") {
            secret = v.trim().to_uppercase();
        } else if let Some(v) = part.strip_prefix("issuer=") {
            issuer = percent_decode(v);
        } else if let Some(v) = part.strip_prefix("accountname=") {
            account = percent_decode(v);
        }
    }

    // label can be "Issuer:Account" — strip the issuer prefix if present
    if let Some((_, after)) = account.split_once(':') {
        account = after.trim().to_string();
    }

    if secret.is_empty() {
        return Err("no `secret` parameter found in otpauth URI".into());
    }

    Ok((secret, issuer, account))
}

/// Minimal percent-decoder — converts %XX escapes and `+` → space.
fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte as char);
                    i += 3;
                    continue;
                }
            }
        } else if bytes[i] == b'+' {
            result.push(' ');
            i += 1;
            continue;
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 test vector: secret = "12345678901234567890" in ASCII → Base32 = GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ
    // At time step 1 (T=59), expected TOTP = 94287082
    // We only test that generate() produces a 6-digit code and parse_otpauth_uri() works.

    // RFC 6238 test secret: ASCII "12345678901234567890" = 20 bytes = 160 bits
    const TEST_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn generate_returns_6_digits() {
        let result = generate(TEST_SECRET).expect("should generate code");
        assert_eq!(result.code.len(), 6, "code must be 6 digits");
        assert!(result.valid_for_secs <= 30, "ttl must be within one period");
        assert_eq!(result.period_secs, 30);
        // Verify code is all digits
        assert!(result.code.chars().all(|c| c.is_ascii_digit()), "code must be numeric");
    }

    #[test]
    fn parse_otpauth_extracts_secret() {
        let uri = format!(
            "otpauth://totp/Example%3Aalice%40example.com?secret={TEST_SECRET}&issuer=Example"
        );
        let (secret, issuer, account) = parse_otpauth_uri(&uri).unwrap();
        assert_eq!(secret, TEST_SECRET);
        assert_eq!(issuer, "Example");
        assert_eq!(account, "alice@example.com");
    }

    #[test]
    fn parse_otpauth_no_issuer_colon() {
        let uri = format!("otpauth://totp/GitHub?secret={TEST_SECRET}&issuer=GitHub");
        let (secret, issuer, account) = parse_otpauth_uri(&uri).unwrap();
        assert_eq!(secret, TEST_SECRET);
        assert_eq!(issuer, "GitHub");
        assert_eq!(account, "GitHub");
    }

    #[test]
    fn parse_otpauth_rejects_invalid() {
        assert!(parse_otpauth_uri("https://example.com").is_err());
        assert!(parse_otpauth_uri("otpauth://totp/Test?issuer=X").is_err());
    }

    #[test]
    fn generate_rejects_invalid_secret() {
        assert!(generate("NOT_BASE32!!!").is_err());
    }

    #[test]
    fn generate_rejects_too_short_secret() {
        // 16 chars = 80 bits, below the 128-bit RFC 6238 minimum
        assert!(generate("JBSWY3DPEHPK3PXP").is_err());
    }
}
