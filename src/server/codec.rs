use rand::Rng;

// ─── Base58 Alphabet (Bitcoin-style) ───────────────────────
//
// Excludes: 0, O, I, l to avoid visual ambiguity.

const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

const CODE_LENGTH: usize = 8;

/// Generate a random 8-character base58 session code.
///
/// Entropy: `log2(58^8) ≈ 47 bits`.
/// Sufficient for a short-lived session (10-minute TTL) with
/// server-side rate limiting (10 join attempts/minute).
pub fn generate_session_code() -> String {
    let mut rng = rand::thread_rng();
    let mut code = String::with_capacity(CODE_LENGTH);
    for _ in 0..CODE_LENGTH {
        let idx = rng.gen_range(0..BASE58.len());
        code.push(BASE58[idx] as char);
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_length() {
        let code = generate_session_code();
        assert_eq!(code.len(), 8);
    }

    #[test]
    fn test_code_uses_valid_chars() {
        let code = generate_session_code();
        for c in code.chars() {
            assert!(
                BASE58.contains(&(c as u8)),
                "invalid character '{}' in code",
                c
            );
        }
    }

    #[test]
    fn test_codes_are_unique() {
        let mut codes = std::collections::HashSet::new();
        for _ in 0..1000 {
            codes.insert(generate_session_code());
        }
        assert_eq!(codes.len(), 1000);
    }

    #[test]
    fn test_no_ambiguous_chars() {
        let code = generate_session_code();
        assert!(!code.contains('0'), "must not contain '0'");
        assert!(!code.contains('O'), "must not contain 'O'");
        assert!(!code.contains('I'), "must not contain 'I'");
        assert!(!code.contains('l'), "must not contain 'l'");
    }
}
