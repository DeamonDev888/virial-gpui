//! URL encoding for path segments, kept apart from the git plumbing so it can
//! be tested on its own.

/// Percent-encode one path segment for use inside a GitHub URL. GitHub keeps
/// a generous set of characters verbatim; only the truly unsafe ones need
/// escaping (`#`, `?`, `%`, spaces, control characters and non-ASCII).
pub fn encode_segment(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'!' | b'$'
            | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b':' | b'@' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "../../tests/infrastructure/git_url.rs"]
mod tests;