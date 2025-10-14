# Text hierarchy

- **Headers:** Use `header_text` (bold + accent). In markdown docs, keep heading markers as usual.
- **Primary body text:** Default foreground.
- **Secondary text / hints:** Use `secondary_line` / `secondary_span` helpers (`dim`).

# Accent and colors

- **Accent:** Use `ACCENT` (`Color::Indexed(208)`) for selections, key hints, and primary actions. Access via helpers (`selection_style`, `.fg(ACCENT)`), do not hardcode the index in call sites.
- **Success / OK:** `Color::Green`.
- **Errors / warnings:** `Color::Red`.
- **Charts / derived colors:** Prefer helper functions inside `styles.rs` or chart modules; avoid inventing new palettes without discussion.

# Stylize conventions

- Prefer the `Stylize` trait (`"text".dim().bold()`) over manual `Style` construction when possible.
- Convert strings with `.into()` for `Span`/`Line` when the target type is obvious; fall back to `Span::from` / `Line::from` when inference fails.
- Keep chained styling calls on a single line after `rustfmt`. If that causes wrapping, split the expression across multiple lines with a single indent level.

# Layout helpers

- Use `split_vertical` / `centered_rect` from `components::utils` for consistent spacing between sections.
- Reuse `secondary_line` for footers rather than crafting new dimmed paragraphs in-place.

# Avoid

- Avoid ANSI `white`/`black` overrides – rely on theme defaults unless the background color is custom.
- Do not introduce new indexed colors; update `ACCENT` if a palette change is required.
- Skip bespoke padding logic if an existing helper (`split_vertical`, table builders) already handles the layout.
