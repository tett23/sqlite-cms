//! UUIDv7（RFC 9562）。post と article のファイル名の先頭に付け、URL の slug にする（ADR 0058）。
//! 先頭の 48 ビットが作った時刻（UNIX 時間のミリ秒）なので、同じ日付の記事も作った順に並ぶ。

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// 今の時刻の UUIDv7。
pub fn new_v7() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    from_parts(now.as_millis() as u64, random_bits(now.as_nanos()))
}

/// 時刻（ミリ秒）と乱数から UUIDv7 を作る。乱数は下位 74 ビットだけを使う。
pub fn from_parts(unix_ms: u64, random: u128) -> String {
    let rand_a = (random >> 62) & 0xfff;
    let rand_b = random & ((1u128 << 62) - 1);
    let value = (u128::from(unix_ms & 0xffff_ffff_ffff) << 80) | (0x7 << 76) | (rand_a << 64) | (0b10 << 62) | rand_b;
    let hex = format!("{value:032x}");
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

/// 乱数。標準ライブラリの RandomState（プロセスごとに乱数で初期化される SipHash の鍵）で、時刻をかき混ぜる。
/// 暗号に使う乱数ではないが、同じミリ秒に作ったものを区別するには足りる。
fn random_bits(seed: u128) -> u128 {
    let mut out = 0u128;
    for i in 0..2u128 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u128(seed ^ i);
        out = (out << 64) | u128::from(hasher.finish());
    }
    out
}

/// ファイル名（拡張子を除く）が UUIDv7 で始まるなら、その UUIDv7 を返す。
/// UUIDv7 だけか、UUIDv7 の後に `-` で続くもの（`<UUIDv7>-<タイトル>`）を認める。英字は小文字だけ。
pub fn leading_v7(stem: &str) -> Option<&str> {
    let uuid = stem.get(..36)?;
    if !(stem.len() == 36 || stem[36..].starts_with('-')) {
        return None;
    }
    let valid = uuid.char_indices().all(|(i, c)| match i {
        8 | 13 | 18 | 23 => c == '-',
        14 => c == '7',
        19 => matches!(c, '8' | '9' | 'a' | 'b'),
        _ => c.is_ascii_digit() || ('a'..='f').contains(&c),
    });
    valid.then_some(uuid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_follows_rfc_9562() {
        // RFC 9562 の UUIDv7 の例（時刻 0x017F22E279B0、乱数 0xCC3 と 0x18C4DC0C0C07398F）。
        let random = (0xcc3u128 << 62) | 0x18c4_dc0c_0c07_398f;
        assert_eq!(from_parts(0x017f_22e2_79b0, random), "017f22e2-79b0-7cc3-98c4-dc0c0c07398f");
    }

    #[test]
    fn new_ids_are_valid_distinct_and_ordered_by_time() {
        let a = new_v7();
        let b = new_v7();
        assert_eq!(leading_v7(&a), Some(a.as_str()));
        assert_ne!(a, b);
        // 時刻が違えば、文字列の順が時刻の順になる。
        assert!(from_parts(1_000, u128::MAX) < from_parts(1_001, 0));
    }

    #[test]
    fn leading_v7_accepts_only_v7_at_the_start() {
        let id = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";
        assert_eq!(leading_v7(id), Some(id));
        assert_eq!(leading_v7(&format!("{id}-SQLite-と-CMS")), Some(id));
        for stem in [
            "",
            "hello",
            "2026-09-26",
            &format!("{id}x"),
            "017F22E2-79B0-7CC3-98C4-DC0C0C07398F",
            "017f22e2-79b0-4cc3-98c4-dc0c0c07398f",
            "017f22e2-79b0-7cc3-c8c4-dc0c0c07398f",
            "017f22e2_79b0-7cc3-98c4-dc0c0c07398f",
            "017f22e2-79b0-7cc3-98c4-dc0c0c07398g",
        ] {
            assert_eq!(leading_v7(stem), None, "{stem}");
        }
    }
}
