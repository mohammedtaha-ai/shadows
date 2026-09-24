//! One job: reading Claude's `/context` answer (spec §12.8).
//!
//! Sent as a prompt, `/context` answers one markdown message whose table
//! "Estimated usage by category" (`| Category | Tokens | Percentage |`) is the
//! breakdown. Everything after that table — per-tool, per-agent, per-skill —
//! is ignored.

/// One category of the context breakdown.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
#[schema(as = ContextCategory)]
pub struct Category {
    pub name: String,
    pub tokens: u64,
    pub percent: f64,
}

/// The rows of the category table, in its order; `None` when the answer has
/// no such table or a row cannot be read.
pub fn parse(markdown: &str) -> Option<Vec<Category>> {
    let mut lines = markdown.lines().map(str::trim);
    lines.find(|line| {
        let cells = cells(line);
        cells.len() == 3
            && cells[0].eq_ignore_ascii_case("category")
            && cells[1].eq_ignore_ascii_case("tokens")
    })?;
    let mut categories = Vec::new();
    for line in lines {
        if !line.starts_with('|') {
            break;
        }
        let cells = cells(line);
        if cells
            .iter()
            .all(|c| c.chars().all(|ch| matches!(ch, '-' | ':')))
        {
            continue; // the separator row
        }
        let [name, tokens, percent] = cells.as_slice() else {
            return None;
        };
        categories.push(Category {
            name: name.to_string(),
            tokens: tokens_of(tokens)?,
            percent: percent.trim_end_matches('%').trim().parse().ok()?,
        });
    }
    (!categories.is_empty()).then_some(categories)
}

fn cells(line: &str) -> Vec<&str> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(str::trim).collect()
}

/// `756`, `3.8k`, `923.9k`, `1m`, `1.2m`.
fn tokens_of(text: &str) -> Option<u64> {
    let text = text.trim().to_ascii_lowercase();
    let (number, scale) = match text.chars().last()? {
        'k' => (&text[..text.len() - 1], 1_000.0),
        'm' => (&text[..text.len() - 1], 1_000_000.0),
        _ => (text.as_str(), 1.0),
    };
    let value: f64 = number.trim().replace(',', "").parse().ok()?;
    Some((value * scale).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_counts_read_their_suffixes() {
        assert_eq!(tokens_of("756"), Some(756));
        assert_eq!(tokens_of("3.8k"), Some(3_800));
        assert_eq!(tokens_of("923.9k"), Some(923_900));
        assert_eq!(tokens_of("1m"), Some(1_000_000));
        assert_eq!(tokens_of("lots"), None);
    }

    #[test]
    fn a_table_without_a_separator_row_is_read_too() {
        let md = "| Category | Tokens | Percentage |\n| Messages | 3.8k | 0.4% |";
        assert_eq!(parse(md).unwrap()[0].tokens, 3_800);
    }
}
