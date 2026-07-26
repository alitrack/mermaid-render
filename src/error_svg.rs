//! Error SVG rendering — returns a styled error diagram instead of panicking.
//! Inspired by ariel-rs and mermaid.js browser error format.

/// Render a parse/rendering error as a styled SVG.
pub fn render_error_svg(message: &str) -> String {
    let escaped = xml_escape(message);
    format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="120" viewBox="0 0 400 120">"#,
            r##"<rect width="400" height="120" fill="#fff5f5" rx="8"/>"##,
            r##"<rect width="400" height="120" fill="none" stroke="#e53e3e" stroke-width="2" rx="8"/>"##,
            r##"<text x="200" y="38" font-family="monospace" font-size="13" fill="#c53030" text-anchor="middle" font-weight="bold">Syntax error</text>"##,
            r##"<text x="200" y="62" font-family="monospace" font-size="11" fill="#e53e3e" text-anchor="middle">{}</text>"##,
            r##"<text x="200" y="86" font-family="monospace" font-size="10" fill="#a0aec0" text-anchor="middle">mermaid-render</text>"##,
            r#"</svg>"#,
        ),
        escaped
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
