/// XML utility functions for safe SVG generation.

fn is_valid_xml_char(c: char) -> bool {
    matches!(
        c as u32,
        0x09 | 0x0A | 0x0D | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF
    )
}

pub fn sanitize_xml_text(text: &str) -> String {
    text.chars().filter(|&c| is_valid_xml_char(c)).collect()
}

pub fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if !is_valid_xml_char(c) { continue; }
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}
