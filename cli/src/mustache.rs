//! Mustache のような書き方で、値を Markdown に埋め込む（ADR 0043）。
//!
//! 対応する書き方は次のとおり。区切りを変える書き方（`{{=<% %>=}}`）には対応しない。
//!
//! | 書き方 | 意味 |
//! |---|---|
//! | `{{name}}` | 値を文字として埋め込む（Markdown の記号をエスケープする） |
//! | `{{{name}}}`、`{{& name}}` | 値をそのまま埋め込む（Markdown として読まれる） |
//! | `{{#name}}…{{/name}}` | 値があれば中身を描く。中では、その値の中の名前も使える |
//! | `{{^name}}…{{/name}}` | 値がなければ中身を描く |
//! | `{{! 注釈}}` | 何も描かない |
//! | `{{> name}}` | 部品（パーシャル）を置く。一行に単独で書く |
//!
//! 名前は `license.name` のように `.` でたどれる。`{{.}}` はいまの値を表す。
//! 値のない名前と空の文字列は「値がない」とみなし、埋め込むと空になる。
//! セクション、注釈、パーシャルのタグだけの行は、行ごと取り除く（Mustache の standalone の規則）。

use std::collections::BTreeMap;
use std::fmt;

/// 埋め込む値。
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    String(String),
    Map(BTreeMap<String, Value>),
}

impl Value {
    fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Map(_) => true,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<Option<String>> for Value {
    fn from(s: Option<String>) -> Self {
        s.map_or(Value::Null, Value::String)
    }
}

/// 書き誤り。line は 1 から数えた行。
#[derive(Debug, PartialEq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} 行目: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, PartialEq)]
enum Node {
    Text(String),
    Variable { name: String, escape: bool },
    Section { name: String, inverted: bool, children: Vec<Node> },
    Partial { name: String, line: usize },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TagKind {
    Escaped,
    Raw,
    Section,
    Inverted,
    Close,
    Comment,
    Partial,
}

impl TagKind {
    /// 一行に単独で書いたときに、行ごと取り除くタグか。
    fn can_stand_alone(self) -> bool {
        matches!(self, TagKind::Section | TagKind::Inverted | TagKind::Close | TagKind::Comment | TagKind::Partial)
    }
}

enum Token {
    Text(String),
    Tag { kind: TagKind, name: String, line: usize },
}

fn error(line: usize, message: impl Into<String>) -> Error {
    Error { line, message: message.into() }
}

fn line_of(template: &str, pos: usize) -> usize {
    template[..pos].matches('\n').count() + 1
}

/// タグの名前を確かめる。英数字、`_`、`-`、`.` だけを使える。
fn check_name(name: &str, line: usize, tag: &str) -> Result<(), Error> {
    let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if valid {
        Ok(())
    } else {
        Err(error(line, format!("タグの名前がありません、または使えない文字を含みます: {tag}")))
    }
}

fn tokenize(template: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while let Some(offset) = template[pos..].find("{{") {
        let start = pos + offset;
        let line = line_of(template, start);
        let (kind, inner_start, close) = match template[start + 2..].chars().next() {
            Some('{') => (TagKind::Raw, start + 3, "}}}"),
            Some('&') => (TagKind::Raw, start + 3, "}}"),
            Some('#') => (TagKind::Section, start + 3, "}}"),
            Some('^') => (TagKind::Inverted, start + 3, "}}"),
            Some('/') => (TagKind::Close, start + 3, "}}"),
            Some('!') => (TagKind::Comment, start + 3, "}}"),
            Some('>') => (TagKind::Partial, start + 3, "}}"),
            Some('=') => return Err(error(line, "区切りを変える書き方（{{=…=}}）には対応していません")),
            _ => (TagKind::Escaped, start + 2, "}}"),
        };
        let Some(inner_len) = template[inner_start..].find(close) else {
            return Err(error(line, format!("タグが閉じていません（{close} がありません）")));
        };
        let end = inner_start + inner_len + close.len();
        let tag = &template[start..end];
        let name = template[inner_start..inner_start + inner_len].trim().to_string();
        if kind != TagKind::Comment {
            check_name(&name, line, tag)?;
        }

        let mut text = template[pos..start].to_string();
        let mut next = end;
        if kind.can_stand_alone() {
            // タグの前が行頭からの空白だけで、後が行末までの空白だけなら、行ごと取り除く。
            let line_start = text.rfind('\n').map_or(0, |i| i + 1);
            // 前の文字列に改行がなければ、前のタグの直後が行頭（テンプレートの先頭か、改行の直後）でなければならない。
            let at_line_start = text.contains('\n') || template[..pos].is_empty() || template[..pos].ends_with('\n');
            let before_is_blank = at_line_start && text[line_start..].chars().all(|c| c == ' ' || c == '\t');
            let rest = &template[end..];
            let after_len = rest.find('\n').map_or(rest.len(), |i| i + 1);
            let after_is_blank = rest[..after_len].trim_end_matches(['\n', '\r']).chars().all(|c| c == ' ' || c == '\t');
            if before_is_blank && after_is_blank {
                text.truncate(line_start);
                next = end + after_len;
            } else if kind == TagKind::Partial {
                return Err(error(line, format!("パーシャルは一行に単独で書いてください: {tag}")));
            }
        }
        if !text.is_empty() {
            tokens.push(Token::Text(text));
        }
        if kind != TagKind::Comment {
            tokens.push(Token::Tag { kind, name, line });
        }
        pos = next;
    }
    if pos < template.len() {
        tokens.push(Token::Text(template[pos..].to_string()));
    }
    Ok(tokens)
}

fn parse(tokens: Vec<Token>) -> Result<Vec<Node>, Error> {
    // 開いているセクションの名前と行と、その中身。
    let mut stack: Vec<(String, bool, usize, Vec<Node>)> = vec![(String::new(), false, 0, Vec::new())];
    for token in tokens {
        let node = match token {
            Token::Text(text) => Node::Text(text),
            Token::Tag { kind, name, line } => match kind {
                TagKind::Escaped => Node::Variable { name, escape: true },
                TagKind::Raw => Node::Variable { name, escape: false },
                TagKind::Partial => Node::Partial { name, line },
                TagKind::Section | TagKind::Inverted => {
                    stack.push((name, kind == TagKind::Inverted, line, Vec::new()));
                    continue;
                }
                TagKind::Close => {
                    let (open, inverted, _, children) = stack.pop().expect("先頭は常にある");
                    if stack.is_empty() {
                        return Err(error(line, format!("開いていないセクションを閉じています: {{{{/{name}}}}}")));
                    }
                    if open != name {
                        return Err(error(line, format!("{{{{#{open}}}}} を {{{{/{name}}}}} で閉じています")));
                    }
                    Node::Section { name, inverted, children }
                }
                TagKind::Comment => unreachable!("注釈は字句にしない"),
            },
        };
        stack.last_mut().expect("先頭は常にある").3.push(node);
    }
    let (name, _, line, nodes) = stack.pop().expect("先頭は常にある");
    if !stack.is_empty() {
        return Err(error(line, format!("セクションが閉じていません: {{{{#{name}}}}}")));
    }
    Ok(nodes)
}

fn lookup<'a>(stack: &[&'a Value], name: &str) -> Option<&'a Value> {
    if name == "." {
        return stack.last().copied();
    }
    let mut parts = name.split('.');
    let first = parts.next()?;
    let mut value = stack.iter().rev().find_map(|context| match context {
        Value::Map(map) => map.get(first),
        _ => None,
    })?;
    for part in parts {
        value = match value {
            Value::Map(map) => map.get(part)?,
            _ => return None,
        };
    }
    Some(value)
}

/// 値を Markdown の中に文字として置けるよう、記号をバックスラッシュでエスケープする。
/// 強調、コード、リンク、HTML、数式、表、取り消し線に使う記号を対象にする。
pub fn escape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '&' | '$' | '~' | '|' | '!' | '{' | '}') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn render_nodes(
    nodes: &[Node],
    stack: &mut Vec<&Value>,
    partials: &BTreeMap<String, String>,
    out: &mut String,
) -> Result<(), Error> {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Variable { name, escape } => {
                let text = match lookup(stack, name) {
                    Some(Value::String(s)) => s.clone(),
                    _ => String::new(),
                };
                out.push_str(&if *escape { escape_markdown(&text) } else { text });
            }
            Node::Section { name, inverted, children } => {
                let value = lookup(stack, name);
                let truthy = value.is_some_and(Value::is_truthy);
                if *inverted {
                    if !truthy {
                        render_nodes(children, stack, partials, out)?;
                    }
                } else if let Some(value) = value.filter(|v| v.is_truthy()) {
                    stack.push(value);
                    render_nodes(children, stack, partials, out)?;
                    stack.pop();
                }
            }
            Node::Partial { name, line } => {
                let Some(partial) = partials.get(name) else {
                    let known = partials.keys().map(String::as_str).collect::<Vec<_>>().join("、");
                    return Err(error(*line, format!("知らないパーシャルです: {name}（使えるもの: {known}）")));
                };
                out.push_str(partial);
            }
        }
    }
    Ok(())
}

/// テンプレートに値を埋め込む。partials はパーシャルの名前と、そこに置く文字列。
pub fn render(template: &str, data: &Value, partials: &BTreeMap<String, String>) -> Result<String, Error> {
    let nodes = parse(tokenize(template)?)?;
    let mut out = String::new();
    render_nodes(&nodes, &mut vec![data], partials, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Value {
        Value::Map(BTreeMap::from([
            ("title".to_string(), Value::from("A & B")),
            ("empty".to_string(), Value::from("")),
            ("none".to_string(), Value::Null),
            (
                "license".to_string(),
                Value::Map(BTreeMap::from([
                    ("name".to_string(), Value::from("CC0 1.0")),
                    ("url".to_string(), Value::from("https://example.com/a_b")),
                ])),
            ),
        ]))
    }

    fn partials() -> BTreeMap<String, String> {
        BTreeMap::from([("search".to_string(), "<search>\n".to_string())])
    }

    fn r(template: &str) -> String {
        render(template, &data(), &partials()).unwrap()
    }

    fn err(template: &str) -> Error {
        render(template, &data(), &partials()).unwrap_err()
    }

    #[test]
    fn variables_escape_markdown_and_triple_braces_do_not() {
        assert_eq!(r("# {{title}}"), "# A \\& B");
        assert_eq!(r("{{ title }}"), "A \\& B");
        assert_eq!(r("{{{title}}}"), "A & B");
        assert_eq!(r("{{& title}}"), "A & B");
        assert_eq!(escape_markdown("*a* _b_ [c](d) <e> `f` $g$ ~h~ |i| !{j}\\"), "\\*a\\* \\_b\\_ \\[c\\](d) \\<e\\> \\`f\\` \\$g\\$ \\~h\\~ \\|i\\| \\!\\{j\\}\\\\");
    }

    #[test]
    fn dotted_names_and_missing_values() {
        assert_eq!(r("[{{license.name}}]({{{license.url}}})"), "[CC0 1.0](https://example.com/a_b)");
        assert_eq!(r("<{{missing}}{{none}}{{license.missing}}{{title.name}}>"), "<>");
    }

    #[test]
    fn sections_render_when_the_value_is_present() {
        assert_eq!(r("{{#license}}{{name}}/{{title}}{{/license}}"), "CC0 1.0/A \\& B");
        assert_eq!(r("{{#title}}[{{.}}]{{/title}}"), "[A \\& B]");
        for missing in ["empty", "none", "missing"] {
            assert_eq!(r(&format!("{{{{#{missing}}}}}x{{{{/{missing}}}}}")), "", "{missing}");
            assert_eq!(r(&format!("{{{{^{missing}}}}}x{{{{/{missing}}}}}")), "x", "{missing}");
        }
        assert_eq!(r("{{^license}}x{{/license}}"), "");
        assert_eq!(r("{{#license}}{{#url}}<{{{.}}}>{{/url}}{{/license}}"), "<https://example.com/a_b>");
    }

    #[test]
    fn standalone_tags_remove_their_lines() {
        assert_eq!(r("a\n{{#license}}\nb\n{{/license}}\nc\n"), "a\nb\nc\n");
        assert_eq!(r("a\n  {{! 注釈 }}  \nb"), "a\nb");
        assert_eq!(r("{{^none}}\r\nx\r\n{{/none}}\r\n"), "x\r\n");
        // 行にほかのものがあれば、行は残す。
        assert_eq!(r("a {{#license}}b{{/license}}\n"), "a b\n");
        assert_eq!(r("{{title}}{{#license}}\nx{{/license}}"), "A \\& B\nx");
        assert_eq!(r("a{{! 注釈 }}b"), "ab");
    }

    #[test]
    fn partials_must_stand_alone() {
        assert_eq!(r("a\n{{> search}}\nb"), "a\n<search>\nb");
        assert_eq!(r("  {{>search}}  "), "<search>\n");
        assert_eq!(err("a {{> search}}"), Error { line: 1, message: "パーシャルは一行に単独で書いてください: {{> search}}".into() });
        assert_eq!(
            err("\n{{> nav}}"),
            Error { line: 2, message: "知らないパーシャルです: nav（使えるもの: search）".into() }
        );
    }

    #[test]
    fn errors_report_the_line() {
        assert_eq!(err("a\n{{title"), Error { line: 2, message: "タグが閉じていません（}} がありません）".into() });
        assert_eq!(err("{{{title}}"), Error { line: 1, message: "タグが閉じていません（}}} がありません）".into() });
        assert_eq!(err("{{#a}}\n{{#b}}\n{{/a}}"), Error { line: 3, message: "{{#b}} を {{/a}} で閉じています".into() });
        assert_eq!(err("\n\n{{#a}}x"), Error { line: 3, message: "セクションが閉じていません: {{#a}}".into() });
        assert_eq!(err("x{{/a}}"), Error { line: 1, message: "開いていないセクションを閉じています: {{/a}}".into() });
        assert_eq!(err("{{=<% %>=}}"), Error { line: 1, message: "区切りを変える書き方（{{=…=}}）には対応していません".into() });
        assert_eq!(err("{{}}"), Error { line: 1, message: "タグの名前がありません、または使えない文字を含みます: {{}}".into() });
        assert_eq!(err("{{a b}}"), Error { line: 1, message: "タグの名前がありません、または使えない文字を含みます: {{a b}}".into() });
    }

    #[test]
    fn text_without_tags_is_unchanged() {
        assert_eq!(r(""), "");
        assert_eq!(r("{ } }} {x}\n"), "{ } }} {x}\n");
        assert_eq!(r("日本語の{{title}}"), "日本語のA \\& B");
    }
}
