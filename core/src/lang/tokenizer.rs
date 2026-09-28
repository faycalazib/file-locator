//! Multilingual word tokenizer (goal.md §5bis-B).
//!
//! Tantivy's `SimpleTokenizer` splits on every non alphanumeric char, which
//! cuts Arabic words in pieces at each vowel mark (tashkeel is a combining
//! mark, not a letter). This tokenizer keeps combining marks inside words and
//! emits a *normalized* token text while keeping the byte offsets of the
//! original text, so highlighting stays exact.

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

use super::normalize::{fold, lowercase_strip_arabic_marks};

/// How token text is normalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Folding {
    /// Lowercase + remove all diacritics (Latin accents, Arabic tashkeel) +
    /// unify Arabic letter variants. Used by the generic `body` field:
    /// "resume" finds "résumé", "عقد" finds "عَقْد".
    Full,
    /// Lowercase + remove Arabic marks only; Latin accents are kept because the
    /// Snowball stemmers expect them ("contrats" → "contrat").
    StemInput,
}

#[derive(Clone)]
pub struct WordTokenizer {
    folding: Folding,
    token: Token,
}

impl WordTokenizer {
    pub fn new(folding: Folding) -> Self {
        Self { folding, token: Token::default() }
    }
}

/// Letters, digits, combining marks and connector punctuation ("_") form words.
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || is_combining_mark(c)
}

fn is_combining_mark(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}' // Latin combining diacritics
        | '\u{0610}'..='\u{061A}'
        | '\u{064B}'..='\u{065F}' // Arabic tashkeel
        | '\u{0670}'
        | '\u{06D6}'..='\u{06ED}'
    )
}

pub struct WordTokenStream<'a> {
    text: &'a str,
    cursor: usize,
    folding: Folding,
    token: &'a mut Token,
}

impl Tokenizer for WordTokenizer {
    type TokenStream<'a> = WordTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> WordTokenStream<'a> {
        self.token.reset();
        WordTokenStream { text, cursor: 0, folding: self.folding, token: &mut self.token }
    }
}

impl TokenStream for WordTokenStream<'_> {
    fn advance(&mut self) -> bool {
        let rest = &self.text[self.cursor..];
        let Some(start_rel) = rest.find(is_word_char) else {
            self.cursor = self.text.len();
            return false;
        };
        let start = self.cursor + start_rel;
        let end = self.text[start..]
            .find(|c: char| !is_word_char(c))
            .map_or(self.text.len(), |e| start + e);
        self.cursor = end;

        let raw = &self.text[start..end];
        self.token.text.clear();
        match self.folding {
            Folding::Full => self.token.text.push_str(&fold(raw)),
            Folding::StemInput => self.token.text.push_str(&lowercase_strip_arabic_marks(raw)),
        }
        self.token.offset_from = start;
        self.token.offset_to = end;
        self.token.position = self.token.position.wrapping_add(1);
        // A word made only of marks folds to nothing: skip it.
        if self.token.text.is_empty() {
            return self.advance();
        }
        true
    }

    fn token(&self) -> &Token {
        self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        self.token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(folding: Folding, text: &str) -> Vec<(String, usize, usize)> {
        let mut tokenizer = WordTokenizer::new(folding);
        let mut stream = tokenizer.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            let t = stream.token();
            out.push((t.text.clone(), t.offset_from, t.offset_to));
        }
        out
    }

    #[test]
    fn folds_latin_accents_and_keeps_offsets() {
        let t = tokens(Folding::Full, "Le Résumé, l’été");
        assert_eq!(t[0].0, "le");
        assert_eq!(t[1].0, "resume");
        assert_eq!(&"Le Résumé, l’été"[t[1].1..t[1].2], "Résumé");
        assert_eq!(t.last().unwrap().0, "ete");
    }

    #[test]
    fn keeps_arabic_word_with_tashkeel_whole() {
        let text = "هذا العَقْدُ مُهِمّ";
        let t = tokens(Folding::Full, text);
        assert_eq!(t.len(), 3);
        assert_eq!(t[1].0, "العقد");
        assert_eq!(&text[t[1].1..t[1].2], "العَقْدُ");
    }

    #[test]
    fn stem_input_keeps_latin_accents() {
        let t = tokens(Folding::StemInput, "Contrats Signés");
        assert_eq!(t[0].0, "contrats");
        assert_eq!(t[1].0, "signés");
    }

    #[test]
    fn underscores_and_digits_stay_in_words() {
        let t = tokens(Folding::Full, "facture_2024-117.xlsx");
        let words: Vec<_> = t.iter().map(|x| x.0.as_str()).collect();
        assert_eq!(words, ["facture_2024", "117", "xlsx"]);
    }
}
