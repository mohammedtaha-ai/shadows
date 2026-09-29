//! One job: one file's text in, its tags out (spec §15.3).

use std::cell::RefCell;

use tree_sitter_tags::TagsContext;

use crate::languages::Language;

thread_local! {
    static CONTEXT: RefCell<TagsContext> = RefCell::new(TagsContext::new());
}

/// `Tag::line_range` stops at 180 bytes, so the signature is cut from the text.
const SIGNATURE_CHARS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Definition,
    Reference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    /// The capture's kind: "function", "method", "class", …
    pub kind: String,
    pub role: Role,
    /// 1-based: the line the name is on.
    pub line: u32,
    /// Definitions only: that line, trimmed, at most 200 characters.
    pub signature: Option<String>,
}

/// Blocking: parses `text`. A file that does not fully parse still gives
/// the tags found. Never panics on any input.
pub fn extract(language: &Language, text: &str) -> Vec<Tag> {
    let source = text.as_bytes();
    let lines: Vec<&str> = text.lines().collect();
    CONTEXT.with_borrow_mut(|ctx| {
        let Ok((tags, _failed)) = ctx.generate_tags(&language.config, source, None) else {
            return Vec::new();
        };
        tags.filter_map(Result::ok)
            .filter_map(|tag| {
                let name = text.get(tag.name_range.clone())?.to_string();
                let kind = language
                    .config
                    .syntax_type_name(tag.syntax_type_id)
                    .to_string();
                // `span` is the name node's position; its start row is the name's line.
                let row = tag.span.start.row;
                let line = u32::try_from(row + 1).ok()?;
                let signature = if tag.is_definition {
                    lines
                        .get(row)
                        .map(|l| l.trim().chars().take(SIGNATURE_CHARS).collect())
                } else {
                    None
                };
                let role = if tag.is_definition {
                    Role::Definition
                } else {
                    Role::Reference
                };
                Some(Tag {
                    name,
                    kind,
                    role,
                    line,
                    signature,
                })
            })
            .collect()
    })
}
