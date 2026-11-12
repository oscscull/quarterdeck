//! The "Ask Claude" chat so far, written out for the next prompt.

use serde::Deserialize;

/// One earlier message in the chat, with the view it was sent from.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatLine {
    /// `you` or `claude`.
    pub from: String,
    pub page: String,
    pub text: String,
}

/// The last `keep` messages, oldest first, or a note that this is the first.
pub fn render(lines: &[ChatLine], keep: usize) -> String {
    let start = lines.len().saturating_sub(keep);
    let recent = lines.get(start..).unwrap_or_default();
    if recent.is_empty() {
        return "(This is the first message.)".to_owned();
    }
    recent
        .iter()
        .map(|l| {
            let who = if l.from == "you" { "User" } else { "You" };
            format!("{who} (on {}): {}", l.page, l.text)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(from: &str, text: &str) -> ChatLine {
        ChatLine {
            from: from.to_owned(),
            page: "Today".to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn keeps_only_the_latest_messages() {
        let lines = [line("you", "a"), line("claude", "b"), line("you", "c")];
        assert_eq!(render(&lines, 2), "You (on Today): b\n\nUser (on Today): c");
    }

    #[test]
    fn says_when_there_is_no_history() {
        assert_eq!(render(&[], 20), "(This is the first message.)");
    }
}
