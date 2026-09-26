const TYPES: &[(&str, &str)] = &[
    ("html", "text/html; charset=utf-8"),
    ("htm", "text/html; charset=utf-8"),
    ("css", "text/css; charset=utf-8"),
    ("js", "text/javascript; charset=utf-8"),
    ("mjs", "text/javascript; charset=utf-8"),
    ("txt", "text/plain; charset=utf-8"),
    ("md", "text/markdown; charset=utf-8"),
    ("csv", "text/csv; charset=utf-8"),
    ("json", "application/json"),
    ("map", "application/json"),
    ("xml", "application/xml"),
    ("wasm", "application/wasm"),
    ("sqlite", "application/vnd.sqlite3"),
    ("pdf", "application/pdf"),
    ("zip", "application/zip"),
    ("svg", "image/svg+xml"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("avif", "image/avif"),
    ("ico", "image/x-icon"),
    ("bmp", "image/bmp"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
    ("ttf", "font/ttf"),
    ("otf", "font/otf"),
    ("mp3", "audio/mpeg"),
    ("ogg", "audio/ogg"),
    ("wav", "audio/wav"),
    ("mp4", "video/mp4"),
    ("webm", "video/webm"),
];

pub fn by_extension(extension: &str) -> Option<&'static str> {
    TYPES
        .iter()
        .find(|(ext, _)| ext.eq_ignore_ascii_case(extension))
        .map(|(_, mime)| *mime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_extensions() {
        assert_eq!(by_extension("html"), Some("text/html; charset=utf-8"));
        assert_eq!(by_extension("wasm"), Some("application/wasm"));
        assert_eq!(by_extension("svg"), Some("image/svg+xml"));
        assert_eq!(by_extension("sqlite"), Some("application/vnd.sqlite3"));
    }

    #[test]
    fn extension_is_case_insensitive() {
        assert_eq!(by_extension("PNG"), Some("image/png"));
        assert_eq!(by_extension("Jpeg"), Some("image/jpeg"));
    }

    #[test]
    fn unknown_extension_is_none() {
        assert_eq!(by_extension("zzz"), None);
        assert_eq!(by_extension(""), None);
    }

    #[test]
    fn text_types_declare_utf8() {
        for (ext, mime) in TYPES {
            if mime.starts_with("text/") {
                assert!(mime.ends_with("; charset=utf-8"), "{ext}");
            }
        }
    }
}
