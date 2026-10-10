use super::*;

#[test]
fn safe_characters_stay_verbatim() {
    assert_eq!(encode_segment("Cargo.toml"), "Cargo.toml");
    assert_eq!(encode_segment("src"), "src");
    assert_eq!(encode_segment("my-file_v2~"), "my-file_v2~");
    assert_eq!(encode_segment("a!$&'()*+,;=:@b"), "a!$&'()*+,;=:@b");
}

#[test]
fn unsafe_characters_are_percent_encoded() {
    assert_eq!(encode_segment("a b"), "a%20b");
    assert_eq!(encode_segment("100%"), "100%25");
    assert_eq!(encode_segment("q&a?"), "q&a%3F");
    assert_eq!(encode_segment("#frag"), "%23frag");
    assert_eq!(encode_segment("café"), "caf%C3%A9");
    assert_eq!(encode_segment("👋"), "%F0%9F%91%8B");
}

#[test]
fn empty_segments_stay_empty() {
    assert_eq!(encode_segment(""), "");
}
