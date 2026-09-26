const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(STANDARD[(n >> (18 - 6 * i) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
pub fn encode_url_safe_no_pad(bytes: &[u8]) -> String {
    encode(bytes).replace('+', "-").replace('/', "_").trim_end_matches('=').to_string()
}

fn url_safe_value(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some(u32::from(c - b'A')),
        b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

pub fn decode_url_safe(s: &str) -> Option<Vec<u8>> {
    let s = s.trim_end_matches('=').as_bytes();
    if s.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for chunk in s.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= url_safe_value(c)? << (18 - 6 * i);
        }
        let bytes = n.to_be_bytes();
        out.extend_from_slice(&bytes[1..chunk.len()]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC4648: [(&str, &str); 7] = [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];

    #[test]
    fn encodes_rfc4648_test_vectors() {
        for (plain, encoded) in RFC4648 {
            assert_eq!(encode(plain.as_bytes()), encoded, "{plain:?}");
        }
    }

    #[test]
    fn encodes_all_byte_values_with_standard_alphabet() {
        assert_eq!(encode(&[0xfb, 0xff, 0xbf]), "+/+/");
        assert_eq!(encode(&[0, 0, 0]), "AAAA");
    }

    #[test]
    fn decodes_rfc4648_test_vectors_with_or_without_padding() {
        for (plain, encoded) in RFC4648 {
            assert_eq!(decode_url_safe(encoded).unwrap(), plain.as_bytes(), "{encoded:?}");
            assert_eq!(decode_url_safe(encoded.trim_end_matches('=')).unwrap(), plain.as_bytes());
        }
    }

    #[test]
    fn decodes_url_safe_alphabet() {
        assert_eq!(decode_url_safe("-_-_").unwrap(), [0xfb, 0xff, 0xbf]);
    }

    #[test]
    fn rejects_invalid_input() {
        assert_eq!(decode_url_safe("Zm9v+"), None);
        assert_eq!(decode_url_safe("Zm9v/g"), None);
        assert_eq!(decode_url_safe("Z"), None);
        assert_eq!(decode_url_safe("Zm 9v"), None);
    }

    #[test]
    fn round_trips_arbitrary_bytes() {
        let bytes: Vec<u8> = (0..=255).collect();
        assert_eq!(decode_url_safe(&encode_url_safe_no_pad(&bytes)).unwrap(), bytes);
    }
}
