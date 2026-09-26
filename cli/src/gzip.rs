//! gzip（RFC 1952）で圧縮する。`serve` の応答を本番（Cloudflare）と同じく圧縮して配信するために使う（ADR 0026）。
//!
//! 中身は DEFLATE（RFC 1951）の固定ハフマン符号のブロック一つと、LZ77 による繰り返しの置き換え。
//! 動的ハフマン符号を作らないぶん zlib より圧縮率は少し劣るが、手元のプレビューには十分で、実装が小さい。

const WINDOW_SIZE: usize = 1 << 15;
const WINDOW_MASK: usize = WINDOW_SIZE - 1;
const HASH_BITS: u32 = 15;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
/// 一致を探すときにたどる候補の数。多いほど圧縮率は上がり、遅くなる。
const MAX_CHAIN: usize = 64;
/// これ以上の長さの一致が見つかれば、探すのをやめる。
const GOOD_ENOUGH: usize = 128;

/// 長さの符号（257〜285）の、各符号が表す最小の長さと追加ビット数。
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];

/// 距離の符号（0〜29）の、各符号が表す最小の距離と追加ビット数。
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/// 下位のビットから詰めていく書き込み口（DEFLATE のビット順）。
struct BitWriter {
    out: Vec<u8>,
    buffer: u64,
    count: u32,
}

impl BitWriter {
    fn new(out: Vec<u8>) -> Self {
        BitWriter { out, buffer: 0, count: 0 }
    }

    /// 値の下位 bits ビットを、下位から書く。
    fn write(&mut self, value: u32, bits: u32) {
        self.buffer |= u64::from(value) << self.count;
        self.count += bits;
        while self.count >= 8 {
            self.out.push(self.buffer as u8);
            self.buffer >>= 8;
            self.count -= 8;
        }
    }

    /// ハフマン符号は上位のビットから書く決まりなので、ビットを反転して書く。
    fn write_code(&mut self, code: u32, bits: u32) {
        self.write(code.reverse_bits() >> (32 - bits), bits);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.count > 0 {
            self.out.push(self.buffer as u8);
        }
        self.out
    }
}

/// 固定ハフマン符号で、リテラルか長さの記号（0〜287）を書く。
fn write_literal_or_length(writer: &mut BitWriter, symbol: u32) {
    match symbol {
        0..=143 => writer.write_code(0x30 + symbol, 8),
        144..=255 => writer.write_code(0x190 + symbol - 144, 9),
        256..=279 => writer.write_code(symbol - 256, 7),
        _ => writer.write_code(0xC0 + symbol - 280, 8),
    }
}

fn write_match(writer: &mut BitWriter, length: usize, distance: usize) {
    let index = LENGTH_BASE.iter().rposition(|&base| usize::from(base) <= length).unwrap();
    write_literal_or_length(writer, 257 + index as u32);
    writer.write((length - usize::from(LENGTH_BASE[index])) as u32, u32::from(LENGTH_EXTRA[index]));

    let index = DISTANCE_BASE.iter().rposition(|&base| usize::from(base) <= distance).unwrap();
    writer.write_code(index as u32, 5);
    writer.write((distance - usize::from(DISTANCE_BASE[index])) as u32, u32::from(DISTANCE_EXTRA[index]));
}

fn hash(data: &[u8], pos: usize) -> usize {
    let value = (u32::from(data[pos]) << 16) | (u32::from(data[pos + 1]) << 8) | u32::from(data[pos + 2]);
    (value.wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize
}

/// 同じ 3 バイトで始まる直前の位置を連鎖させて、最長の一致を探す。
struct Matcher {
    head: Vec<usize>,
    prev: Vec<usize>,
}

const NONE: usize = usize::MAX;

impl Matcher {
    fn new() -> Self {
        Matcher { head: vec![NONE; 1 << HASH_BITS], prev: vec![NONE; WINDOW_SIZE] }
    }

    fn insert(&mut self, data: &[u8], pos: usize) {
        if pos + MIN_MATCH > data.len() {
            return;
        }
        let h = hash(data, pos);
        self.prev[pos & WINDOW_MASK] = self.head[h];
        self.head[h] = pos;
    }

    /// pos から始まる最長の一致の（長さ、距離）。MIN_MATCH 未満なら長さ 0。
    fn longest(&self, data: &[u8], pos: usize) -> (usize, usize) {
        if pos + MIN_MATCH > data.len() {
            return (0, 0);
        }
        let max = (data.len() - pos).min(MAX_MATCH);
        let (mut best_length, mut best_distance) = (0, 0);
        let mut candidate = self.head[hash(data, pos)];
        let mut chain = 0;
        while candidate != NONE && pos - candidate <= WINDOW_SIZE && chain < MAX_CHAIN {
            if data[candidate + best_length.min(max - 1)] == data[pos + best_length.min(max - 1)] {
                let length = data[candidate..].iter().zip(&data[pos..pos + max]).take_while(|(a, b)| a == b).count();
                if length > best_length {
                    best_length = length;
                    best_distance = pos - candidate;
                    if length >= max || length >= GOOD_ENOUGH {
                        break;
                    }
                }
            }
            let next = self.prev[candidate & WINDOW_MASK];
            // 連鎖は窓の大きさで循環するので、古い位置に戻ったら終える。
            if next == NONE || next >= candidate {
                break;
            }
            candidate = next;
            chain += 1;
        }
        if best_length < MIN_MATCH {
            (0, 0)
        } else {
            (best_length, best_distance)
        }
    }
}

/// DEFLATE で圧縮する（固定ハフマン符号のブロック一つ）。
fn deflate(data: &[u8], out: Vec<u8>) -> Vec<u8> {
    let mut writer = BitWriter::new(out);
    writer.write(1, 1); // 最後のブロック
    writer.write(1, 2); // 固定ハフマン符号
    let mut matcher = Matcher::new();
    let mut pos = 0;
    while pos < data.len() {
        let (length, distance) = matcher.longest(data, pos);
        // 一つ先の位置でもっと長い一致があれば、ここはリテラルにする（遅延評価）。
        let next = if length > 0 && length < GOOD_ENOUGH && pos + 1 < data.len() {
            matcher.insert(data, pos);
            let next = matcher.longest(data, pos + 1);
            if next.0 > length {
                write_literal_or_length(&mut writer, u32::from(data[pos]));
                pos += 1;
                continue;
            }
            true
        } else {
            false
        };
        if length == 0 {
            matcher.insert(data, pos);
            write_literal_or_length(&mut writer, u32::from(data[pos]));
            pos += 1;
            continue;
        }
        write_match(&mut writer, length, distance);
        let start = if next { pos + 1 } else { pos };
        for p in start..pos + length {
            matcher.insert(data, p);
        }
        pos += length;
    }
    write_literal_or_length(&mut writer, 256); // ブロックの終わり
    writer.finish()
}

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (n, entry) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *entry = c;
    }
    table
}

pub fn crc32(data: &[u8]) -> u32 {
    let table = crc32_table();
    !data.iter().fold(!0u32, |crc, &byte| table[((crc ^ u32::from(byte)) & 0xFF) as usize] ^ (crc >> 8))
}

/// gzip の形式で圧縮する。
pub fn compress(data: &[u8]) -> Vec<u8> {
    // ID1 ID2 CM(deflate) FLG MTIME(4) XFL OS(不明)
    let header = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
    let mut out = deflate(data, header);
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// システムの gzip で展開する（macOS と CI の Ubuntu に入っている）。
    fn gunzip(data: &[u8]) -> Vec<u8> {
        let mut child = Command::new("gzip")
            .arg("-dc")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("gzip を起動できません");
        child.stdin.take().unwrap().write_all(data).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "gzip が展開できません: {}", String::from_utf8_lossy(&output.stderr));
        output.stdout
    }

    fn pseudo_random(len: usize) -> Vec<u8> {
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
    }

    #[test]
    fn crc32_matches_known_values() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"The quick brown fox jumps over the lazy dog"), 0x414F_A339);
    }

    #[test]
    fn round_trips_through_system_gzip() {
        let javascript = "export function greet(name) {\n  return `こんにちは、${name}さん`;\n}\n".repeat(200);
        let long_run = vec![b'a'; 100_000];
        let mixed: Vec<u8> = (0..70_000u32).map(|i| (i % 251) as u8).collect();
        let inputs: Vec<Vec<u8>> = vec![
            Vec::new(),
            b"a".to_vec(),
            b"abc".to_vec(),
            b"abcabcabcabcabc".to_vec(),
            javascript.clone().into_bytes(),
            long_run,
            mixed,
            pseudo_random(50_000),
            (0..=255u8).collect(),
        ];
        for input in inputs {
            assert_eq!(gunzip(&compress(&input)), input, "長さ {}", input.len());
        }
    }

    #[test]
    fn compresses_repetitive_text() {
        let javascript = "export function greet(name) { return `Hello, ${name}`; }\n".repeat(1000);
        let compressed = compress(javascript.as_bytes());
        assert!(compressed.len() * 20 < javascript.len(), "{} → {}", javascript.len(), compressed.len());
    }

    #[test]
    fn long_matches_across_the_window_round_trip() {
        // 窓（32 KiB）を超える距離の繰り返しは参照できない。窓をまたぐ入力でも正しく展開できること。
        let block = pseudo_random(40_000);
        let input = [block.clone(), block].concat();
        assert_eq!(gunzip(&compress(&input)), input);
    }
}
