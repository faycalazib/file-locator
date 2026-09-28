//! Text normalization shared by indexing, querying and highlighting.

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

const TATWEEL: char = '\u{0640}';

fn is_arabic_mark(c: char) -> bool {
    matches!(c, '\u{0610}'..='\u{061A}' | '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{06D6}'..='\u{06ED}')
}

/// Unifies Arabic letter variants that users type interchangeably.
fn unify_arabic_letter(c: char) -> char {
    match c {
        'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
        'ى' => 'ي',
        'ة' => 'ه',
        'ؤ' => 'و',
        'ئ' => 'ي',
        _ => c,
    }
}

/// Full folding: lowercase, no diacritics (Latin or Arabic), no tatweel,
/// unified Arabic letters. "Résumé" → "resume", "العَقْدُ" → "العقد".
pub fn fold(text: &str) -> String {
    text.nfd()
        .filter(|&c| !is_combining_mark(c) && c != TATWEEL)
        .flat_map(char::to_lowercase)
        .map(unify_arabic_letter)
        .collect::<String>()
        .nfc()
        .collect()
}

/// Input of the Snowball stemmers: lowercase, Arabic marks and tatweel
/// removed, Latin accents kept.
pub fn lowercase_strip_arabic_marks(text: &str) -> String {
    text.chars()
        .filter(|&c| !is_arabic_mark(c) && c != TATWEEL)
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_french_spanish_arabic() {
        assert_eq!(fold("Résumé"), "resume");
        assert_eq!(fold("CANCIÓN"), "cancion");
        assert_eq!(fold("Ñandú"), "nandu");
        assert_eq!(fold("العَقْدُ"), "العقد");
        assert_eq!(fold("إيجار"), "ايجار");
        assert_eq!(fold("مدرسة"), "مدرسه");
        assert_eq!(fold("عـــقد"), "عقد");
    }

    #[test]
    fn stem_input_keeps_accents() {
        assert_eq!(lowercase_strip_arabic_marks("Été"), "été");
        assert_eq!(lowercase_strip_arabic_marks("العَقْدُ"), "العقد");
    }
}
