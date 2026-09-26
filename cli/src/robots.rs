//! robots.txt（ADR 0037）。記事リポジトリの content/robots.txt があればそれを、なければ既定の内容を配信する。

/// 既定の robots.txt。すべてのクローラにすべてのページのクロールを許可する。`init` もこれを content/robots.txt に書く。
pub const DEFAULT: &str = "\
# クローラへの指示。sqlite-cms が /robots.txt として配信する。
# すべてのクローラに、すべてのページのクロールを許可している。
# 特定のクローラを断るときは、たとえば次のように書く。
#
# User-agent: GPTBot
# Disallow: /

User-agent: *
Allow: /
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_allows_every_crawler() {
        let directives: Vec<&str> =
            DEFAULT.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')).collect();
        assert_eq!(directives, ["User-agent: *", "Allow: /"]);
    }
}
