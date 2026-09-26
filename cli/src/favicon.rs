/// サイト名の最初の一文字を描いた、仮の favicon（SVG）。
pub fn placeholder(title: &str) -> String {
    let initial = title.trim().chars().next().map_or('?', |c| c.to_ascii_uppercase());
    let escaped = match initial {
        '&' => "&amp;".to_string(),
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        c => c.to_string(),
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <rect width="64" height="64" rx="8" fill="#000000"/>
  <text x="32" y="45" font-family="sans-serif" font-size="38" font-weight="bold" text-anchor="middle" fill="#ffffff">{escaped}</text>
</svg>
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initial_of(svg: &str) -> &str {
        let start = svg.find("fill=\"#ffffff\">").unwrap() + "fill=\"#ffffff\">".len();
        &svg[start..svg[start..].find("</text>").unwrap() + start]
    }

    #[test]
    fn draws_the_first_character_of_the_title() {
        assert_eq!(initial_of(&placeholder("tett23の記事置き場")), "T");
        assert_eq!(initial_of(&placeholder("記事置き場")), "記");
        assert_eq!(initial_of(&placeholder("  my blog")), "M");
    }

    #[test]
    fn escapes_markup_characters() {
        assert_eq!(initial_of(&placeholder("<script>")), "&lt;");
        assert_eq!(initial_of(&placeholder("&co")), "&amp;");
    }

    #[test]
    fn empty_title_falls_back_to_a_question_mark() {
        assert_eq!(initial_of(&placeholder("")), "?");
    }

    #[test]
    fn is_an_svg_document() {
        let svg = placeholder("t");
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.trim_end().ends_with("</svg>"));
    }
}
