//! gzip（RFC 1952）で圧縮する。`serve` の応答を本番（Cloudflare）と同じく圧縮して配信するために使う（ADR 0026）。
//!
//! 中身は DEFLATE（RFC 1951）の動的ハフマン符号のブロックと、LZ77 による繰り返しの置き換え（ADR 0026、0038）。

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

/// LZ77 の結果。リテラルか、（長さ、距離）の一致。
#[derive(Clone, Copy)]
enum Token {
    Literal(u8),
    Match { length: u16, distance: u16 },
}

/// 長さを、長さの記号（257〜285）の番号と追加ビットにする。
fn length_code(length: usize) -> (usize, u32, u32) {
    let index = LENGTH_BASE.iter().rposition(|&base| usize::from(base) <= length).unwrap();
    (index, u32::from(LENGTH_EXTRA[index]), (length - usize::from(LENGTH_BASE[index])) as u32)
}

/// 距離を、距離の記号（0〜29）と追加ビットにする。
fn distance_code(distance: usize) -> (usize, u32, u32) {
    let index = DISTANCE_BASE.iter().rposition(|&base| usize::from(base) <= distance).unwrap();
    (index, u32::from(DISTANCE_EXTRA[index]), (distance - usize::from(DISTANCE_BASE[index])) as u32)
}

/// LZ77 で、データをリテラルと一致の並びにする。
fn tokenize(data: &[u8]) -> Vec<Token> {
    let mut tokens = Vec::with_capacity(data.len() / 2);
    let mut matcher = Matcher::new();
    let mut pos = 0;
    while pos < data.len() {
        let (length, distance) = matcher.longest(data, pos);
        // 一つ先の位置でもっと長い一致があれば、ここはリテラルにする（遅延評価）。
        let next = if length > 0 && length < GOOD_ENOUGH && pos + 1 < data.len() {
            matcher.insert(data, pos);
            let next = matcher.longest(data, pos + 1);
            if next.0 > length {
                tokens.push(Token::Literal(data[pos]));
                pos += 1;
                continue;
            }
            true
        } else {
            false
        };
        if length == 0 {
            matcher.insert(data, pos);
            tokens.push(Token::Literal(data[pos]));
            pos += 1;
            continue;
        }
        tokens.push(Token::Match { length: length as u16, distance: distance as u16 });
        let start = if next { pos + 1 } else { pos };
        for p in start..pos + length {
            matcher.insert(data, p);
        }
        pos += length;
    }
    tokens
}

/// 出現回数から、各記号の符号の長さ（ハフマン符号）を求める。長さが max_bits を超えたら、回数をならして作り直す。
fn huffman_lengths(freqs: &[u32], max_bits: u8) -> Vec<u8> {
    let mut weights: Vec<u64> = freqs.iter().map(|&f| u64::from(f)).collect();
    loop {
        let lengths = build_lengths(&weights);
        if lengths.iter().all(|&l| l <= max_bits) {
            return lengths;
        }
        for weight in weights.iter_mut().filter(|w| **w > 0) {
            *weight = (*weight >> 1).max(1);
        }
    }
}

fn build_lengths(weights: &[u64]) -> Vec<u8> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let mut lengths = vec![0u8; weights.len()];
    let used: Vec<usize> = (0..weights.len()).filter(|&i| weights[i] > 0).collect();
    if used.len() == 1 {
        lengths[used[0]] = 1;
        return lengths;
    }
    // 葉は記号、内部の節は後ろに足す。parent で親をたどって深さを求める。
    let mut parent: Vec<usize> = vec![usize::MAX; weights.len()];
    let mut heap: BinaryHeap<Reverse<(u64, usize)>> = used.iter().map(|&i| Reverse((weights[i], i))).collect();
    while heap.len() > 1 {
        let Reverse((a_weight, a)) = heap.pop().unwrap();
        let Reverse((b_weight, b)) = heap.pop().unwrap();
        let node = parent.len();
        parent.push(usize::MAX);
        parent[a] = node;
        parent[b] = node;
        heap.push(Reverse((a_weight + b_weight, node)));
    }
    for &symbol in &used {
        let mut depth = 0u8;
        let mut node = symbol;
        while parent[node] != usize::MAX {
            node = parent[node];
            depth = depth.saturating_add(1);
        }
        lengths[symbol] = depth;
    }
    lengths
}

/// 符号の長さから、正準ハフマン符号を作る。
fn canonical_codes(lengths: &[u8]) -> Vec<u32> {
    let max = lengths.iter().copied().max().unwrap_or(0) as usize;
    let mut count = vec![0u32; max + 1];
    for &l in lengths.iter().filter(|&&l| l > 0) {
        count[l as usize] += 1;
    }
    let mut next = vec![0u32; max + 2];
    let mut code = 0u32;
    for bits in 1..=max {
        code = (code + count[bits - 1]) << 1;
        next[bits] = code;
    }
    lengths
        .iter()
        .map(|&l| {
            if l == 0 {
                return 0;
            }
            let c = next[l as usize];
            next[l as usize] += 1;
            c
        })
        .collect()
}

/// 少なくとも二つの記号が使われるようにする（一つだけの符号は扱いにくい展開器がある）。
fn ensure_two_symbols(freqs: &mut [u32]) {
    let used = freqs.iter().filter(|&&f| f > 0).count();
    for f in freqs.iter_mut().filter(|f| **f == 0).take(2usize.saturating_sub(used)) {
        *f = 1;
    }
}

/// 符号の長さの並びを、符号の長さの記号（0〜18）と追加ビットの並びにする（16：直前を繰り返す、17、18：0 を繰り返す）。
fn run_length_encode(lengths: &[u8]) -> Vec<(u8, u32, u32)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lengths.len() {
        let value = lengths[i];
        let mut run = lengths[i..].iter().take_while(|&&l| l == value).count();
        i += run;
        if value == 0 {
            while run >= 11 {
                let n = run.min(138);
                out.push((18, 7, (n - 11) as u32));
                run -= n;
            }
            if run >= 3 {
                out.push((17, 3, (run - 3) as u32));
                run = 0;
            }
            out.extend(std::iter::repeat_n((0, 0, 0), run));
        } else {
            out.push((value, 0, 0));
            run -= 1;
            while run >= 3 {
                let n = run.min(6);
                out.push((16, 2, (n - 3) as u32));
                run -= n;
            }
            out.extend(std::iter::repeat_n((value, 0, 0), run));
        }
    }
    out
}

/// 符号の長さの記号を書く順（RFC 1951 3.2.7）。
const CODE_LENGTH_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// 一つのブロックの記号の数。ブロックごとに符号を作り直し、データの偏りの変化に合わせる。
const BLOCK_TOKENS: usize = 1 << 16;

/// 動的ハフマン符号のブロック一つを書く。
fn write_block(writer: &mut BitWriter, tokens: &[Token], last: bool) {
    let mut litlen_freqs = [0u32; 286];
    let mut dist_freqs = [0u32; 30];
    for token in tokens {
        match *token {
            Token::Literal(byte) => litlen_freqs[byte as usize] += 1,
            Token::Match { length, distance } => {
                litlen_freqs[257 + length_code(length as usize).0] += 1;
                dist_freqs[distance_code(distance as usize).0] += 1;
            }
        }
    }
    litlen_freqs[256] += 1;
    ensure_two_symbols(&mut litlen_freqs);
    ensure_two_symbols(&mut dist_freqs);

    let litlen_lengths = huffman_lengths(&litlen_freqs, 15);
    let dist_lengths = huffman_lengths(&dist_freqs, 15);
    let litlen_codes = canonical_codes(&litlen_lengths);
    let dist_codes = canonical_codes(&dist_lengths);

    let hlit = (257..=286).rev().find(|&n| litlen_lengths[n - 1] > 0).unwrap_or(257).max(257);
    let hdist = (1..=30).rev().find(|&n| dist_lengths[n - 1] > 0).unwrap_or(1).max(1);
    let mut all_lengths = litlen_lengths[..hlit].to_vec();
    all_lengths.extend_from_slice(&dist_lengths[..hdist]);
    let encoded = run_length_encode(&all_lengths);

    let mut cl_freqs = [0u32; 19];
    for &(symbol, _, _) in &encoded {
        cl_freqs[symbol as usize] += 1;
    }
    ensure_two_symbols(&mut cl_freqs);
    let cl_lengths = huffman_lengths(&cl_freqs, 7);
    let cl_codes = canonical_codes(&cl_lengths);
    let hclen = (4..=19).rev().find(|&n| cl_lengths[CODE_LENGTH_ORDER[n - 1]] > 0).unwrap_or(4);

    writer.write(u32::from(last), 1);
    writer.write(2, 2); // 動的ハフマン符号
    writer.write((hlit - 257) as u32, 5);
    writer.write((hdist - 1) as u32, 5);
    writer.write((hclen - 4) as u32, 4);
    for &symbol in &CODE_LENGTH_ORDER[..hclen] {
        writer.write(u32::from(cl_lengths[symbol]), 3);
    }
    for &(symbol, extra_bits, extra) in &encoded {
        writer.write_code(cl_codes[symbol as usize], u32::from(cl_lengths[symbol as usize]));
        writer.write(extra, extra_bits);
    }

    for token in tokens {
        match *token {
            Token::Literal(byte) => writer.write_code(litlen_codes[byte as usize], u32::from(litlen_lengths[byte as usize])),
            Token::Match { length, distance } => {
                let (index, bits, extra) = length_code(length as usize);
                writer.write_code(litlen_codes[257 + index], u32::from(litlen_lengths[257 + index]));
                writer.write(extra, bits);
                let (index, bits, extra) = distance_code(distance as usize);
                writer.write_code(dist_codes[index], u32::from(dist_lengths[index]));
                writer.write(extra, bits);
            }
        }
    }
    writer.write_code(litlen_codes[256], u32::from(litlen_lengths[256])); // ブロックの終わり
}

/// DEFLATE で圧縮する（動的ハフマン符号のブロック）。
fn deflate(data: &[u8], out: Vec<u8>) -> Vec<u8> {
    let mut writer = BitWriter::new(out);
    let tokens = tokenize(data);
    if tokens.is_empty() {
        write_block(&mut writer, &[], true);
    } else {
        let blocks: Vec<&[Token]> = tokens.chunks(BLOCK_TOKENS).collect();
        for (i, block) in blocks.iter().enumerate() {
            write_block(&mut writer, block, i + 1 == blocks.len());
        }
    }
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
    fn many_blocks_round_trip() {
        // 記号が BLOCK_TOKENS 個を超えると、ブロックを分ける。
        let input = pseudo_random(BLOCK_TOKENS * 3 + 100);
        assert_eq!(gunzip(&compress(&input)), input);
    }

    #[test]
    fn huffman_lengths_are_limited_and_complete() {
        // フィボナッチ数の出現回数は、制限しなければ符号がいちばん深くなる。
        let mut freqs = vec![1u32, 1];
        while freqs.len() < 30 {
            freqs.push(freqs[freqs.len() - 1] + freqs[freqs.len() - 2]);
        }
        for max_bits in [7u8, 15] {
            let lengths = huffman_lengths(&freqs, max_bits);
            assert!(lengths.iter().all(|&l| (1..=max_bits).contains(&l)), "{lengths:?}");
            // 符号の長さが、過不足のない接頭符号を作れること（クラフトの等式）。
            let kraft: f64 = lengths.iter().map(|&l| 0.5f64.powi(i32::from(l))).sum();
            assert!((kraft - 1.0).abs() < 1e-9, "{kraft}");
        }
    }

    #[test]
    fn long_matches_across_the_window_round_trip() {
        // 窓（32 KiB）を超える距離の繰り返しは参照できない。窓をまたぐ入力でも正しく展開できること。
        let block = pseudo_random(40_000);
        let input = [block.clone(), block].concat();
        assert_eq!(gunzip(&compress(&input)), input);
    }
}
