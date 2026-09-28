//! Languages: detection and the Tantivy analyzers (goal.md §5bis-B).

mod normalize;
mod tokenizer;

pub use normalize::fold;
pub use tokenizer::{Folding, WordTokenizer};

use serde::{Deserialize, Serialize};
use tantivy::tokenizer::{Language, RemoveLongFilter, Stemmer, TextAnalyzer, TokenStream, TokenizerManager};

/// Languages with a dedicated stemmed field. Anything else is `Und`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocLang {
    En,
    Fr,
    Es,
    Ar,
    Und,
}

impl DocLang {
    pub const STEMMED: [DocLang; 4] = [DocLang::En, DocLang::Fr, DocLang::Es, DocLang::Ar];

    pub fn code(self) -> &'static str {
        match self {
            DocLang::En => "en",
            DocLang::Fr => "fr",
            DocLang::Es => "es",
            DocLang::Ar => "ar",
            DocLang::Und => "und",
        }
    }

    pub fn from_code(code: &str) -> Option<DocLang> {
        match code {
            "en" => Some(DocLang::En),
            "fr" => Some(DocLang::Fr),
            "es" => Some(DocLang::Es),
            "ar" => Some(DocLang::Ar),
            "und" => Some(DocLang::Und),
            _ => None,
        }
    }

    fn stemmer_language(self) -> Option<Language> {
        match self {
            DocLang::En => Some(Language::English),
            DocLang::Fr => Some(Language::French),
            DocLang::Es => Some(Language::Spanish),
            DocLang::Ar => Some(Language::Arabic),
            DocLang::Und => None,
        }
    }

    /// Name of the analyzer registered for this language's stemmed field.
    pub fn analyzer_name(self) -> String {
        format!("stem_{}", self.code())
    }
}

/// Detects the main language of a text (first 20 000 chars are enough).
pub fn detect(text: &str) -> DocLang {
    let sample: String = text.chars().take(20_000).collect();
    match whatlang::detect(&sample) {
        Some(info) if info.confidence() > 0.2 || sample.len() > 400 => match info.lang() {
            whatlang::Lang::Eng => DocLang::En,
            whatlang::Lang::Fra => DocLang::Fr,
            whatlang::Lang::Spa => DocLang::Es,
            whatlang::Lang::Ara => DocLang::Ar,
            _ => DocLang::Und,
        },
        _ => DocLang::Und,
    }
}

/// Name of the generic analyzer (field `body`, `name`).
pub const GENERIC: &str = "generic";

/// Words longer than this are noise (hashes, base64…).
const MAX_TOKEN_LEN: usize = 64;

pub fn generic_analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(WordTokenizer::new(Folding::Full)).filter(RemoveLongFilter::limit(MAX_TOKEN_LEN)).build()
}

pub fn stem_analyzer(lang: DocLang) -> TextAnalyzer {
    let base = TextAnalyzer::builder(WordTokenizer::new(Folding::StemInput)).filter(RemoveLongFilter::limit(MAX_TOKEN_LEN));
    match lang.stemmer_language() {
        Some(language) => base.filter(Stemmer::new(language)).build(),
        None => base.build(),
    }
}

/// Registers every analyzer on an index's tokenizer manager.
pub fn register(manager: &TokenizerManager) {
    manager.register(GENERIC, generic_analyzer());
    for lang in DocLang::STEMMED {
        manager.register(&lang.analyzer_name(), stem_analyzer(lang));
    }
}

/// Runs an analyzer and returns the token texts.
pub fn analyze(analyzer: &mut TextAnalyzer, text: &str) -> Vec<String> {
    let mut stream = analyzer.token_stream(text);
    let mut out = Vec::new();
    while stream.advance() {
        out.push(stream.token().text.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_four_languages() {
        assert_eq!(detect("Le présent contrat de prestation prend effet à la date de signature par les deux parties."), DocLang::Fr);
        assert_eq!(detect("El presente contrato de prestación de servicios se regirá por la legislación española vigente."), DocLang::Es);
        assert_eq!(detect("This service agreement takes effect on the date it is signed by both parties."), DocLang::En);
        assert_eq!(detect("يبدأ سريان هذا العقد اعتبارًا من تاريخ توقيعه من قبل الطرفين"), DocLang::Ar);
    }

    #[test]
    fn stemmers_group_word_forms() {
        let mut fr = stem_analyzer(DocLang::Fr);
        assert_eq!(analyze(&mut fr, "contrats"), analyze(&mut fr, "contrat"));
        let mut es = stem_analyzer(DocLang::Es);
        assert_eq!(analyze(&mut es, "contratos"), analyze(&mut es, "contrato"));
        let mut en = stem_analyzer(DocLang::En);
        assert_eq!(analyze(&mut en, "agreements"), analyze(&mut en, "agreement"));
        let mut ar = stem_analyzer(DocLang::Ar);
        assert_eq!(analyze(&mut ar, "العقود"), analyze(&mut ar, "عقود"));
    }
}
