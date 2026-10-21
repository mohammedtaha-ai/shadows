use super::{Recorder, Source};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(super) fn table_owners() -> BTreeMap<String, String> {
    include_str!("table-owners.txt")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (table, owner) = line.split_once(" :: ").expect("table :: owner");
            (table.to_owned(), owner.to_owned())
        })
        .collect()
}

// Unsupported SQL constructs become explicit review findings.
pub(super) fn statement(sql: &str) -> bool {
    let first = sql
        .trim_start()
        .split(|ch: char| !ch.is_ascii_alphabetic())
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let lower = sql.to_ascii_lowercase();
    match first.as_str() {
        "insert" | "replace" => lower.contains("into"),
        "delete" => lower.contains("from"),
        "update" => lower.contains("set"),
        "begin" | "commit" | "rollback" => matches!(
            lower.split(';').next().unwrap_or("").trim(),
            "begin"
                | "begin transaction"
                | "begin immediate"
                | "begin exclusive"
                | "begin deferred"
                | "begin immediate transaction"
                | "begin exclusive transaction"
                | "begin deferred transaction"
                | "commit"
                | "commit transaction"
                | "rollback"
                | "rollback transaction"
        ),
        "select" | "with" | "pragma" | "create" | "alter" | "drop" => true,
        _ => false,
    }
}

fn tokens(sql: &str) -> Result<Vec<String>, &'static str> {
    let mut chars = sql.chars().peekable();
    let mut out = Vec::new();
    while let Some(ch) = chars.next() {
        match ch {
            '\'' => {
                let mut closed = false;
                while let Some(ch) = chars.next() {
                    if ch == '\'' {
                        if chars.peek() == Some(&'\'') {
                            chars.next();
                        } else {
                            closed = true;
                            break;
                        }
                    }
                }
                if !closed {
                    return Err("unterminated SQL string");
                }
                out.push("<value>".to_owned());
            }
            '"' | '`' | '[' => {
                let end = if ch == '[' { ']' } else { ch };
                let mut word = String::new();
                let mut closed = false;
                while let Some(ch) = chars.next() {
                    if ch == end {
                        if end != ']' && chars.peek() == Some(&end) {
                            chars.next();
                            word.push(end);
                            continue;
                        }
                        closed = true;
                        break;
                    }
                    word.push(ch);
                }
                if !closed {
                    return Err("unterminated identifier");
                }
                // Quoted identifiers cannot act as clause keywords.
                out.push(format!("@{}", word.to_lowercase()));
            }
            '-' if chars.peek() == Some(&'-') => {
                for ch in chars.by_ref() {
                    if ch == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => return Err("SQL block comment"),
            '{' | '}' => return Err("interpolated SQL"),
            ch if ch.is_ascii_alphanumeric() || ch == '_' => {
                let mut word = String::from(ch);
                while chars
                    .peek()
                    .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                {
                    word.push(chars.next().unwrap());
                }
                out.push(word.to_lowercase());
            }
            ch if ch.is_whitespace() => {}
            ch => out.push(ch.to_string()),
        }
    }
    Ok(out)
}

pub(super) fn check(
    source: &Source,
    node: Node<'_>,
    sql: &str,
    owners: &BTreeMap<String, String>,
    recorder: &mut Recorder,
) {
    if !(source.service || source.adapter || source.owner == "db") {
        return;
    }
    let first = sql
        .trim_start()
        .split(|ch: char| !ch.is_ascii_alphabetic())
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if !statement(sql) {
        return;
    }
    let review = |recorder: &mut Recorder, reason: &str| {
        recorder.at("ownership", source, node, &format!("SQL review: {reason}"))
    };
    let tokens = match tokens(sql) {
        Ok(tokens) => tokens,
        Err(reason) => {
            review(recorder, reason);
            return;
        }
    };
    if tokens.split(|token| token == ";").any(|statement| {
        statement.first().is_some_and(|first| {
            ![
                "select", "insert", "replace", "update", "delete", "begin", "commit", "rollback",
            ]
            .contains(&first.as_str())
        })
    }) {
        review(recorder, "unsupported statement");
        return;
    }
    let mut tables = BTreeSet::new();
    let mut depth = 0_i32;
    let mut from_depths = BTreeSet::new();
    for (i, token) in tokens.iter().enumerate() {
        if token == "(" {
            depth += 1;
        }
        if token == ")" {
            depth -= 1;
            from_depths.retain(|from| *from <= depth);
        }
        if [
            "where",
            "group",
            "order",
            "limit",
            "returning",
            "set",
            "union",
            ";",
        ]
        .contains(&token.as_str())
        {
            from_depths.remove(&depth);
        }
        let expects_table = ["from", "join", "into"].contains(&token.as_str())
            || (token == "update" && (i == 0 || tokens[i - 1] == ";"))
            || (token == "," && from_depths.contains(&depth));
        if token == "from" {
            from_depths.insert(depth);
        }
        if expects_table {
            let next = tokens.get(i + 1).map(String::as_str).unwrap_or("");
            let next = next.strip_prefix('@').unwrap_or(next);
            if next == "(" {
                continue;
            }
            if next.is_empty()
                || next == "or"
                || !next.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
            {
                review(recorder, "unclassified table expression");
                continue;
            }
            if tokens
                .get(i + 2)
                .is_some_and(|next| next == "." || (next == "(" && token != "into"))
            {
                review(recorder, "qualified table or table function");
                continue;
            }
            tables.insert(next.to_owned());
        }
    }
    if tables.is_empty()
        && !["begin", "commit", "rollback"].contains(&first.as_str())
        && (first != "select"
            || tokens
                .iter()
                .any(|token| ["from", "join"].contains(&token.as_str())))
    {
        review(recorder, "no classified table");
    }
    for table in tables {
        match owners.get(&table) {
            Some(owner) if owner == &source.owner || owner == "db" => {}
            Some(owner) => recorder.at(
                "ownership",
                source,
                node,
                &format!("SQL table {table} belongs to {owner}"),
            ),
            None => review(recorder, &format!("unknown table {table}")),
        }
    }
}
