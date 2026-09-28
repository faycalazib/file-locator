//! Passages (lot 8.2): a document is cut into passages of about
//! [`TARGET_WORDS`] words, at the end of a sentence, at most [`MAX_PASSAGES`]
//! per document (the subject of a document is in its beginning; a book or a
//! log does not cost hours). The file name leads the first passage: it often
//! says what the document is about.

/// Words per passage, aimed at.
pub const TARGET_WORDS: usize = 120;
/// A sentence longer than this is cut (tables, logs without full stops).
const MAX_WORDS: usize = 160;
/// Passages per document.
pub const MAX_PASSAGES: usize = 12;

/// One passage: its place in the document's text (bytes) and what is given
/// to the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Passage {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// End of a sentence: `.`, `!`, `?`, the Arabic `؟`, or a line break.
fn ends_sentence(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '؟' | '\n' | '。')
}

/// The passages of a document's text; `name` (the file name) leads the first.
pub fn passages(name: &str, text: &str) -> Vec<Passage> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut words = 0usize;
    let mut in_word = false;
    let cut = |from: usize, to: usize, out: &mut Vec<Passage>| {
        let body = text[from..to].trim();
        if body.is_empty() || out.len() >= MAX_PASSAGES {
            return;
        }
        let shown = if out.is_empty() && !name.is_empty() { format!("{name}\n{body}") } else { body.to_owned() };
        out.push(Passage { start: from, end: to, text: shown });
    };
    for (i, c) in text.char_indices() {
        if out.len() >= MAX_PASSAGES {
            break;
        }
        if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            in_word = true;
            words += 1;
            start.get_or_insert(i);
        }
        let end = i + c.len_utf8();
        let full = (words >= TARGET_WORDS && ends_sentence(c)) || (words >= MAX_WORDS && c.is_whitespace());
        if full {
            if let Some(from) = start.take() {
                cut(from, end, &mut out);
            }
            words = 0;
        }
    }
    if let Some(from) = start {
        cut(from, text.len(), &mut out);
    }
    if out.is_empty() && !name.is_empty() {
        // A file with no text (an image without OCR text): its name alone.
        out.push(Passage { start: 0, end: 0, text: name.to_owned() });
    }
    out
}

/// How many passages a text gives (the estimate before computing).
pub fn count(text: &str) -> usize {
    let words = text.split_whitespace().count();
    words.div_ceil(TARGET_WORDS).clamp(1, MAX_PASSAGES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_120_words_at_the_end_of_a_sentence() {
        let sentence = "Le locataire paie le loyer chaque mois à la date prévue. ";
        let text = sentence.repeat(60); // 11 words × 60 = 660 words
        let parts = passages("bail.docx", &text);
        assert!(parts.len() >= 5 && parts.len() <= 6, "{}", parts.len());
        assert!(parts[0].text.starts_with("bail.docx\nLe locataire"));
        for p in &parts[..parts.len() - 1] {
            let words = text[p.start..p.end].split_whitespace().count();
            assert!((TARGET_WORDS..TARGET_WORDS + 11).contains(&words), "{words}");
            assert!(text[p.start..p.end].trim_end().ends_with('.'));
        }
        // Everything is covered, in order, without overlap.
        assert_eq!(parts[0].start, 0);
        for w in parts.windows(2) {
            assert!(w[0].end <= w[1].start);
        }
    }

    #[test]
    fn limits_and_edge_cases() {
        // No full stop: cut at 160 words.
        let words = "mot ".repeat(1000);
        let parts = passages("", &words);
        assert!(parts.iter().all(|p| p.text.split_whitespace().count() <= MAX_WORDS));
        // A long document: 12 passages at most.
        let long = "Une phrase courte ici. ".repeat(10_000);
        assert_eq!(passages("livre.pdf", &long).len(), MAX_PASSAGES);
        assert_eq!(count(&long), MAX_PASSAGES);
        // Arabic sentences end with ؟ too.
        let ar = "هل دفع المستأجر الإيجار في موعده المحدد هذا الشهر؟ ".repeat(40);
        assert!(passages("", &ar).len() >= 2);
        // No text: the name alone; nothing at all: nothing.
        assert_eq!(passages("photo.jpg", "   ")[0].text, "photo.jpg");
        assert!(passages("", "").is_empty());
        assert_eq!(count("un deux"), 1);
    }
}
