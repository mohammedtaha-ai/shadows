use pulldown_cmark::{Event, Parser, Tag, TagEnd};
fn append(output: &mut String, count: &mut usize, pending: &mut bool, value: &str) {
    for ch in value.chars() {
        if ch.is_whitespace() {
            *pending = !output.is_empty();
            continue;
        }
        if *pending && *count < 240 {
            output.push(' ');
            *count += 1;
        }
        *pending = false;
        if *count < 240 {
            output.push(ch);
            *count += 1;
        }
    }
}
/// CommonMark supplies parsing; this filter retains at most 240 presentation scalars.
pub fn reply_preview(markdown: &str) -> Option<String> {
    if markdown.len() > 16 * 1024 * 1024 {
        return None;
    }
    let mut text = String::with_capacity(960);
    let mut count = 0;
    let mut pending = false;
    let mut omitted = 0usize;
    let mut hidden_html = 0usize;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::Image { .. }) => omitted += 1,
            Event::End(TagEnd::CodeBlock | TagEnd::Image) => {
                omitted = omitted.saturating_sub(1);
                pending = !text.is_empty();
            }
            _ if omitted > 0 => {}
            Event::InlineHtml(tag) => {
                let lower = tag.to_ascii_lowercase();
                let name = lower
                    .trim()
                    .trim_start_matches('<')
                    .trim_start_matches('/')
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .next()
                    .unwrap_or("");
                if matches!(
                    name,
                    "think" | "analysis" | "reasoning" | "script" | "style"
                ) {
                    if lower.starts_with("</") {
                        hidden_html = hidden_html.checked_sub(1)?;
                    } else if !lower.ends_with("/>") {
                        hidden_html += 1;
                    }
                }
            }
            Event::Html(_) => {}
            _ if hidden_html > 0 => {}
            Event::Text(value) => {
                if value.contains(['<', '>', '[', ']']) {
                    return None;
                }
                append(&mut text, &mut count, &mut pending, &value);
            }
            Event::Code(value) => append(&mut text, &mut count, &mut pending, &value),
            Event::SoftBreak | Event::HardBreak | Event::Rule => pending = !text.is_empty(),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::Item
                | TagEnd::List(_)
                | TagEnd::BlockQuote(_),
            ) => pending = !text.is_empty(),
            Event::Start(_) | Event::End(_) => {}
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {
                return None
            }
            Event::TaskListMarker(_) => {}
        }
    }
    if hidden_html != 0 || text.is_empty() {
        None
    } else {
        Some(text)
    }
}
