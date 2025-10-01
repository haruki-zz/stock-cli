/// Convert a user-facing preset label into a safe filesystem slug.
pub fn sanitize_preset_name(name: &str) -> Option<String> {
    let mut slug = String::new();

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if matches!(ch, ' ' | '-' | '_') {
            slug.push(if ch == ' ' { '_' } else { ch });
        }
    }

    if slug.is_empty() {
        None
    } else {
        Some(slug.to_lowercase())
    }
}
