//! Detectors (lot 6.2): personal and business data found by a pattern and
//! **validated** (check digits, keys), so that random numbers are not
//! reported. Arabic-Indic digits (٠-٩, ۰-۹) count as 0-9.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Detector {
    Iban,
    Bic,
    /// French bank details (RIB key).
    Rib,
    Card,
    Email,
    Phone,
    Ip,
    Amount,
    /// Intra-EU VAT number.
    Vat,
    /// SIREN / SIRET, after the word.
    Siren,
    /// French social security number.
    Nir,
    /// Spanish DNI / NIE.
    Dni,
    /// Machine-readable zone of a passport or an identity card.
    Mrz,
}

impl Detector {
    pub const ALL: [Detector; 13] = [
        Detector::Iban,
        Detector::Bic,
        Detector::Rib,
        Detector::Card,
        Detector::Email,
        Detector::Phone,
        Detector::Ip,
        Detector::Amount,
        Detector::Vat,
        Detector::Siren,
        Detector::Nir,
        Detector::Dni,
        Detector::Mrz,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Detector::Iban => "iban",
            Detector::Bic => "bic",
            Detector::Rib => "rib",
            Detector::Card => "card",
            Detector::Email => "email",
            Detector::Phone => "phone",
            Detector::Ip => "ip",
            Detector::Amount => "amount",
            Detector::Vat => "vat",
            Detector::Siren => "siren",
            Detector::Nir => "nir",
            Detector::Dni => "dni",
            Detector::Mrz => "mrz",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.code() == code)
    }

    fn regex(self) -> &'static Regex {
        &PATTERNS[self as usize]
    }

    /// The reported part of a match: group 1 when the pattern has one
    /// (context words such as "SIRET" are not part of the data).
    fn valid(self, found: &str, whole: &str) -> bool {
        match self {
            Detector::Iban => valid_iban(found),
            Detector::Bic => valid_bic(found, whole),
            Detector::Rib => valid_rib(found),
            Detector::Card => valid_card(found),
            Detector::Email => true,
            Detector::Phone => valid_phone(found),
            Detector::Ip => found.split('.').all(|p| p.parse::<u16>().is_ok_and(|n| n <= 255)),
            Detector::Amount => found.chars().any(|c| digit(c).is_some()),
            Detector::Vat => valid_vat(found),
            Detector::Siren => valid_siren(found),
            Detector::Nir => valid_nir(found),
            Detector::Dni => valid_dni(found),
            Detector::Mrz => valid_mrz(found),
        }
    }
}

/// Separators allowed inside numbers: space, no-break spaces, dot, dash.
const SEP: &str = r"[ \u{a0}\u{202f}.\-]";

static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let d = r"[0-9\u{660}-\u{669}\u{6f0}-\u{6f9}]";
    let money_words = r"EUR|USD|GBP|MAD|DZD|TND|CHF|CAD|DH|Dhs?|euros?|dollars?|dirhams?|dinars?";
    let number = format!(r"(?:{d}{{1,3}}(?:[ \u{{a0}}\u{{202f}}.,']{d}{{3}})+|{d}+)(?:[.,]{d}{{1,2}})?");
    let sources = [
        // IBAN: country, key, then 11 to 30 letters or digits (spaces allowed).
        r"\b[A-Z]{2}[0-9]{2}(?: ?[A-Z0-9]){11,30}\b".to_owned(),
        // BIC: optional context word, then the code (group 1).
        r"(?:(?i:\bBIC\b|\bSWIFT\b)[^A-Za-z0-9\n]{0,15})?\b([A-Z]{6}[A-Z0-9]{2}(?:[A-Z0-9]{3})?)\b".to_owned(),
        // RIB: bank 5, branch 5, account 11, key 2.
        r"\b[0-9]{5} ?[0-9]{5} ?[A-Z0-9]{11} ?[0-9]{2}\b".to_owned(),
        // Card: 13 to 19 digits, grouped by spaces or dashes.
        format!(r"\b(?:{d}[ \-]?){{12,18}}{d}\b"),
        r"\b[A-Za-z0-9][A-Za-z0-9._%+\-]*@[A-Za-z0-9\-]+(?:\.[A-Za-z0-9\-]+)*\.[A-Za-z]{2,24}\b".to_owned(),
        // Phone: international, French / Maghreb national, Spanish, US.
        format!(
            r"(?:\+|\b00){d}{{1,3}}{SEP}?(?:\(0\){SEP}?)?{d}{{1,4}}(?:{SEP}?{d}{{2,4}}){{2,4}}\b|\b0[1-9](?:{SEP}?{d}{{2}}){{4}}\b|\b[6789]{d}{{1,2}}(?:[ .]{d}{{2,3}}){{2,3}}\b|\({d}{{3}}\) ?{d}{{3}}-{d}{{4}}"
        ),
        r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b".to_owned(),
        // Amount: a currency before or after a number.
        format!(
            r"(?:[€$£¥]|\b(?:{money_words})\b\.?) ?{number}|{number} ?(?:[€$£¥]|(?:{money_words})\b|د\.م\.?|دج|درهم|دينار|يورو)"
        ),
        // VAT: FR, ES, DE, BE, IT, NL, LU, PT.
        r"\b(?:FR ?[0-9A-HJ-NP-Z]{2} ?[0-9]{3} ?[0-9]{3} ?[0-9]{3}|ES ?[A-Z0-9][0-9]{7}[A-Z0-9]|DE ?[0-9]{9}|BE ?[01][0-9]{9}|IT ?[0-9]{11}|NL ?[0-9]{9}B[0-9]{2}|LU ?[0-9]{8}|PT ?[0-9]{9})\b".to_owned(),
        // SIREN / SIRET after the word (group 1).
        r"(?i:\bsire[nt]\b)[^0-9\n]{0,12}([0-9]{3} ?[0-9]{3} ?[0-9]{3}(?: ?[0-9]{5})?)\b".to_owned(),
        // NIR: sex, year, month, department (2A / 2B), town, order, key.
        r"\b[12] ?[0-9]{2} ?(?:0[1-9]|1[0-2]|[2-9][0-9]) ?(?:[0-9]{2}|2[AB]) ?[0-9]{3} ?[0-9]{3} ?[0-9]{2}\b".to_owned(),
        r"\b(?:[0-9]{8}|[XYZ][0-9]{7})[ \-]?[A-Z]\b".to_owned(),
        // MRZ: passport line 2 (44), identity card line 1 (30).
        r"\b[A-Z0-9<]{9}[0-9][A-Z<]{3}[0-9]{6}[0-9][MFX<][0-9]{6}[0-9][A-Z0-9<]{14}[0-9<][0-9]\b|\b[AIC][A-Z<][A-Z<]{3}[A-Z0-9<]{9}[0-9<][A-Z0-9<]{15}\b".to_owned(),
    ];
    sources.iter().map(|s| Regex::new(s).expect("detector pattern")).collect()
});

/// 0-9, also in Arabic-Indic and Persian digits.
fn digit(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        '\u{660}'..='\u{669}' => Some(c as u32 - 0x660),
        '\u{6f0}'..='\u{6f9}' => Some(c as u32 - 0x6f0),
        _ => None,
    }
}

fn digits(s: &str) -> Vec<u32> {
    s.chars().filter_map(digit).collect()
}

fn compact(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '<').collect()
}

fn luhn(ds: &[u32]) -> bool {
    let sum: u32 = ds
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| if i % 2 == 1 { if d * 2 > 9 { d * 2 - 9 } else { d * 2 } } else { d })
        .sum();
    !ds.is_empty() && sum.is_multiple_of(10)
}

fn all_same(ds: &[u32]) -> bool {
    ds.windows(2).all(|w| w[0] == w[1])
}

/// `text` (letters = 10..35) modulo 97, digit by digit.
fn mod97(text: &str) -> Option<u32> {
    let mut rest = 0u32;
    for c in text.chars() {
        let value = c.to_digit(36)?;
        rest = if value >= 10 { (rest * 100 + value) % 97 } else { (rest * 10 + value) % 97 };
    }
    Some(rest)
}

/// IBAN lengths of common countries (others: 15 to 34).
fn iban_length(country: &str) -> Option<usize> {
    Some(match country {
        "FR" | "IT" | "MC" => 27,
        "ES" | "SE" | "TN" => 24,
        "DE" | "GB" | "IE" => 22,
        "BE" => 16,
        "NL" => 18,
        "LU" => 20,
        "CH" | "LI" => 21,
        "PT" => 25,
        "MA" => 28,
        "DZ" => 26,
        "AT" => 20,
        _ => return None,
    })
}

fn valid_iban(found: &str) -> bool {
    let iban = compact(found);
    if !(15..=34).contains(&iban.len()) || iban_length(&iban[..2]).is_some_and(|n| n != iban.len()) {
        return false;
    }
    let rearranged = format!("{}{}", &iban[4..], &iban[..4]);
    mod97(&rearranged) == Some(1)
}

const COUNTRIES: &str = "AD AE AL AM AO AR AT AU AZ BA BE BF BG BH BJ BR BY CA CD CG CH CI CL CM CN CO CR CY CZ DE DJ DK DO DZ EC EE EG ES FI FR GA GB GE GH GN GR HK HR HU ID IE IL IN IQ IR IS IT JO JP KE KR KW KZ LB LI LK LT LU LV LY MA MC MD ME MG MK ML MR MT MU MX MY NE NG NL NO NZ OM PE PH PK PL PS PT QA RO RS RU SA SE SG SI SK SN SY TD TG TH TN TR TW UA US UY UZ VE VN YE ZA";

/// Country code, and either a digit in the location / branch code or the
/// words "BIC" / "SWIFT" before it (an upper-case 8-letter word is not a BIC).
fn valid_bic(code: &str, whole: &str) -> bool {
    let country = &code[4..6];
    COUNTRIES.split(' ').any(|c| c == country)
        && (code[6..].chars().any(|c| c.is_ascii_digit()) || whole.len() > code.len())
}

/// French RIB key: 97 - ((89 × bank + 15 × branch + 3 × account) mod 97),
/// letters of the account turned into digits.
fn valid_rib(found: &str) -> bool {
    let rib = compact(found);
    if rib.len() != 23 {
        return false;
    }
    let letter = |c: char| -> Option<u64> {
        if let Some(d) = c.to_digit(10) {
            return Some(u64::from(d));
        }
        let value = c.to_digit(36)? - 9; // A = 1
        Some(u64::from(match value {
            1..=9 => value,
            10..=18 => value - 9,
            _ => value - 17,
        }))
    };
    let number = |s: &str| s.chars().try_fold(0u64, |n, c| Some(n * 10 + letter(c)?));
    let (Some(bank), Some(branch), Some(account), Ok(key)) =
        (number(&rib[..5]), number(&rib[5..10]), number(&rib[10..21]), rib[21..].parse::<u64>())
    else {
        return false;
    };
    97 - (89 * bank + 15 * branch + 3 * (account % 97)) % 97 == key
}

fn valid_card(found: &str) -> bool {
    let ds = digits(found);
    if !(13..=19).contains(&ds.len()) || all_same(&ds) || !luhn(&ds) {
        return false;
    }
    let prefix = |n: usize| ds[..n].iter().fold(0u32, |p, d| p * 10 + d);
    let len = ds.len();
    match ds[0] {
        4 => matches!(len, 13 | 16 | 19),
        5 => (51..=55).contains(&prefix(2)) && len == 16,
        2 => (2221..=2720).contains(&prefix(4)) && len == 16,
        3 => matches!(prefix(2), 34 | 37) && len == 15 || (prefix(2) == 35 && len >= 16) || matches!(prefix(2), 30 | 36 | 38) && len == 14,
        6 => (prefix(4) == 6011 || prefix(2) == 65 || prefix(2) == 62 || (644..=649).contains(&prefix(3))) && len >= 16,
        _ => false,
    }
}

fn valid_phone(found: &str) -> bool {
    let ds = digits(found);
    (9..=15).contains(&ds.len()) && !all_same(&ds[1..])
}

fn valid_vat(found: &str) -> bool {
    let vat = compact(found);
    match &vat[..2] {
        "FR" => {
            let siren: String = vat[4..].to_owned();
            let Ok(n) = siren.parse::<u64>() else { return false };
            match vat[2..4].parse::<u64>() {
                Ok(key) => (12 + 3 * (n % 97)) % 97 == key,
                // Letters: newer keys, not computable.
                Err(_) => luhn(&digits(&siren)),
            }
        }
        "BE" => {
            let Ok(n) = vat[2..].parse::<u64>() else { return false };
            97 - (n / 100) % 97 == n % 100
        }
        _ => true,
    }
}

fn valid_siren(found: &str) -> bool {
    let ds = digits(found);
    matches!(ds.len(), 9 | 14) && !all_same(&ds) && luhn(&ds)
}

/// NIR key: 97 - (13 digits mod 97); Corsica 2A = 19, 2B = 18.
fn valid_nir(found: &str) -> bool {
    let nir = compact(found);
    if nir.len() != 15 {
        return false;
    }
    let (body, key) = nir.split_at(13);
    let body = body.replacen("2A", "19", 1).replacen("2B", "18", 1);
    let (Ok(n), Ok(key)) = (body.parse::<u64>(), key.parse::<u64>()) else { return false };
    let n = if nir.contains("2A") { n - 1_000_000 } else if nir.contains("2B") { n - 2_000_000 } else { n };
    97 - n % 97 == key
}

fn valid_dni(found: &str) -> bool {
    let dni = compact(found);
    let (number, letter) = dni.split_at(dni.len() - 1);
    let number = number.replacen('X', "0", 1).replacen('Y', "1", 1).replacen('Z', "2", 1);
    let Ok(n) = number.parse::<usize>() else { return false };
    "TRWAGMYFPDXBNJZSQVHLCKE".chars().nth(n % 23).is_some_and(|c| letter.starts_with(c))
}

/// ICAO check digit: weights 7, 3, 1; letters 10..35, `<` = 0.
fn mrz_check(field: &str, check: char) -> bool {
    let sum: u32 = field
        .chars()
        .zip([7u32, 3, 1].iter().cycle())
        .map(|(c, w)| w * if c == '<' { 0 } else { c.to_digit(36).unwrap_or(0) })
        .sum();
    check.to_digit(10) == Some(sum % 10) || (check == '<' && sum.is_multiple_of(10))
}

fn valid_mrz(found: &str) -> bool {
    let c: Vec<char> = found.chars().collect();
    let s = |a: usize, b: usize| c[a..b].iter().collect::<String>();
    match c.len() {
        // Passport line 2: number, birth date, expiry date.
        44 => mrz_check(&s(0, 9), c[9]) && mrz_check(&s(13, 19), c[19]) && mrz_check(&s(21, 27), c[27]),
        // Identity card line 1: document number.
        30 => mrz_check(&s(5, 14), c[14]) && c[5..14].iter().any(char::is_ascii_digit),
        _ => false,
    }
}

/// The detectors of a request, ready to scan texts.
#[derive(Clone, Debug, Default)]
pub struct Detectors(Vec<Detector>);

impl Detectors {
    pub fn new(list: &[Detector]) -> Self {
        Self(list.to_vec())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Every validated occurrence, with its detector.
    pub fn find(&self, text: &str) -> Vec<(Detector, Range<usize>)> {
        let mut out = Vec::new();
        for &detector in &self.0 {
            for caps in detector.regex().captures_iter(text) {
                let whole = caps.get(0).expect("match");
                let found = caps.get(1).unwrap_or(whole);
                if detector.valid(found.as_str(), whole.as_str()) {
                    out.push((detector, found.range()));
                }
            }
        }
        out
    }

    /// Occurrences per detector code.
    pub fn count(&self, text: &str) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for (detector, _) in self.find(text) {
            *counts.entry(detector.code().to_owned()).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(detector: Detector, text: &str) -> Vec<String> {
        Detectors::new(&[detector]).find(text).into_iter().map(|(_, r)| text[r].to_owned()).collect()
    }

    #[test]
    fn iban_and_rib_keys() {
        assert_eq!(found(Detector::Iban, "IBAN : FR76 3000 6000 0112 3456 7890 189, merci"), ["FR76 3000 6000 0112 3456 7890 189"]);
        assert_eq!(found(Detector::Iban, "ES9121000418450200051332"), ["ES9121000418450200051332"]);
        assert!(found(Detector::Iban, "FR76 3000 6000 0112 3456 7890 188").is_empty(), "wrong key");
        assert_eq!(found(Detector::Rib, "RIB 30006 00001 12345678901 89"), ["30006 00001 12345678901 89"]);
        assert!(found(Detector::Rib, "30006 00001 12345678901 88").is_empty());
    }

    #[test]
    fn bic_needs_a_digit_or_the_word() {
        assert_eq!(found(Detector::Bic, "BIC : BNPAFRPP"), ["BNPAFRPP"]);
        assert_eq!(found(Detector::Bic, "code DEUTDEFF500"), ["DEUTDEFF500"]);
        assert!(found(Detector::Bic, "BORDEAUX FRANCAIS CONTRATS").is_empty(), "an upper-case word is not a BIC");
    }

    #[test]
    fn cards_luhn_and_brand() {
        assert_eq!(found(Detector::Card, "Carte 4111 1111 1111 1111 exp 12/27"), ["4111 1111 1111 1111"]);
        assert_eq!(found(Detector::Card, "Amex 3782-822463-10005"), ["3782-822463-10005"]);
        assert!(found(Detector::Card, "4111 1111 1111 1112").is_empty(), "Luhn");
        assert!(found(Detector::Card, "commande 1234567812345678").is_empty(), "no brand starts with 1");
        assert_eq!(found(Detector::Card, "٤١١١ ١١١١ ١١١١ ١١١١").len(), 1, "Arabic-Indic digits");
    }

    #[test]
    fn contact_data() {
        assert_eq!(found(Detector::Email, "écrire à jean.dupont+rh@exemple.co.uk."), ["jean.dupont+rh@exemple.co.uk"]);
        assert_eq!(found(Detector::Phone, "Tél. 06 12 34 56 78 ou +33 1 23 45 67 89"), ["06 12 34 56 78", "+33 1 23 45 67 89"]);
        assert_eq!(found(Detector::Phone, "llamar al 612 345 678"), ["612 345 678"]);
        assert!(found(Detector::Phone, "facture 2024-117").is_empty());
        assert_eq!(found(Detector::Ip, "serveur 192.168.1.20, version 10.0.19041.1"), ["192.168.1.20"]);
    }

    #[test]
    fn amounts_with_a_currency() {
        assert_eq!(found(Detector::Amount, "Total : 1 250,50 € puis $99.99 et 300 MAD"), ["1 250,50 €", "$99.99", "300 MAD"]);
        assert_eq!(found(Detector::Amount, "المبلغ 500 درهم"), ["500 درهم"]);
        assert!(found(Detector::Amount, "page 12 sur 30").is_empty());
    }

    #[test]
    fn business_numbers() {
        // SIREN 732 829 320: its VAT key is 44.
        assert_eq!(found(Detector::Vat, "TVA FR44 732829320"), ["FR44 732829320"]);
        assert!(found(Detector::Vat, "FR45 732829320").is_empty());
        assert_eq!(found(Detector::Vat, "BE0403170701"), ["BE0403170701"]);
        assert_eq!(found(Detector::Siren, "SIRET : 732 829 320 00074"), ["732 829 320 00074"]);
        assert!(found(Detector::Siren, "SIREN 123 456 789").is_empty(), "Luhn");
        assert!(found(Detector::Siren, "732 829 320").is_empty(), "not without the word");
    }

    #[test]
    fn identity_numbers() {
        assert_eq!(found(Detector::Nir, "n° sécu 1 85 05 78 006 084 91"), ["1 85 05 78 006 084 91"]);
        assert!(found(Detector::Nir, "1 85 05 78 006 084 90").is_empty());
        assert_eq!(found(Detector::Dni, "DNI 12345678Z, NIE X1234567L"), ["12345678Z", "X1234567L"]);
        assert!(found(Detector::Dni, "12345678A").is_empty());
        // ICAO 9303 specimen, passport line 2.
        assert_eq!(found(Detector::Mrz, "L898902C36UTO7408122F1204159ZE184226B<<<<<10").len(), 1);
        assert!(found(Detector::Mrz, "L898902C37UTO7408122F1204159ZE184226B<<<<<10").is_empty());
    }

    #[test]
    fn counts_per_detector() {
        let all = Detectors::new(&Detector::ALL);
        let counts = all.count("Contact : a@b.fr, c@d.es ; carte 4111 1111 1111 1111.");
        assert_eq!(counts.get("email"), Some(&2));
        assert_eq!(counts.get("card"), Some(&1));
        assert_eq!(Detector::parse("iban"), Some(Detector::Iban));
    }
}
