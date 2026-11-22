//! String collation for the prompt order of facts and episodes.
//!
//! V1 sorted equal-strength, same-day facts with `a.content.localeCompare(b.content, "ja")`
//! (ICU's CLDR `ja` collation, tertiary strength) and same-day episodes with
//! `localeCompare` in the browser's default locale. Ties are common, so the collation decides
//! which facts reach the prompt's top 6 and in which order.
//!
//! **Exact parity:** in the browser, install the real ICU collator with
//! `MemoryManager::set_collator(|a, b| /* Intl.Collator("ja").compare(a, b) */)`.
//!
//! **Fallback:** [`JaCollator`] is a self-contained approximation of ICU `ja` (no ICU data in
//! the core). It reproduces the rules that matter for Japanese facts, checked against Node 22
//! `Intl.Collator("ja")`:
//! * script order: spaces < punctuation/symbols (ICU ranks for ~1500 common symbols) <
//!   digits < Latin < kana < kanji (JIS X 0208 order, then other Han) < other scripts;
//! * kana: hiragana = katakana = half-width katakana, small < normal (tertiary), voicing
//!   marks secondary (か < が < ぱ), `ー` takes the vowel of the preceding kana and the
//!   iteration marks `ゝゞヽヾ` repeat it, both with a lower secondary weight;
//! * Latin: case-insensitive primary, lowercase first, full-width = ASCII, Latin-1 accents
//!   secondary;
//! * strings equal at all three levels compare `Equal` (the stable sort keeps their order, as
//!   with ICU's `0`).
//!
//! Not modeled: full Unicode normalization/decomposition, contractions beyond the kana rules,
//! accents outside Latin-1 (ICU sorts `İx` with `i`, before `iy`; the fallback does not),
//! the tertiary order of expansions against case (ICU: `STRASSE` < `straße`; the fallback:
//! `>`), and the exact ICU order of uncommon symbols. This only matters without the injected
//! `Intl.Collator("ja")`.

use std::cmp::Ordering;
use std::sync::OnceLock;

use super::jis_kanji::{JIS_KANJI, JIS_KANJI_COUNT};
use super::symbol_ranks::SYMBOL_RANKS;

/// String comparison used to order prompt facts and episodes (V1 `String#localeCompare`).
/// Any `Fn(&str, &str) -> Ordering` is a `Collator`. It must be a consistent total preorder
/// (ties allowed).
pub trait Collator {
    /// Compares two strings.
    fn compare(&self, a: &str, b: &str) -> Ordering;
}

impl<F: Fn(&str, &str) -> Ordering> Collator for F {
    fn compare(&self, a: &str, b: &str) -> Ordering {
        self(a, b)
    }
}

/// Built-in approximation of ICU's `ja` collation (see the module docs).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct JaCollator;

impl Collator for JaCollator {
    fn compare(&self, a: &str, b: &str) -> Ordering {
        ja_compare(a, b)
    }
}

/// [`JaCollator`] as a function.
pub fn ja_compare(a: &str, b: &str) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let (ea, eb) = (elements(a), elements(b));
    compare_level(&ea, &eb, |e| e.primary)
        .then_with(|| compare_level(&ea, &eb, |e| u32::from(e.secondary)))
        .then_with(|| compare_level(&ea, &eb, |e| u32::from(e.tertiary)))
}

fn compare_level(a: &[Element], b: &[Element], weight: impl Fn(&Element) -> u32) -> Ordering {
    let wa = a.iter().map(&weight).filter(|&w| w != 0);
    let wb = b.iter().map(&weight).filter(|&w| w != 0);
    wa.cmp(wb)
}

// Primary weight ranges (group base + offset).
const P_SPACE: u32 = 0x0000_1000;
const P_SYMBOL_RANKED: u32 = 0x0000_2000;
const P_SYMBOL_OTHER: u32 = 0x0001_0000;
const P_DIGIT: u32 = 0x0100_0000;
const P_NUMERIC_OTHER: u32 = 0x0110_0000;
const P_LATIN: u32 = 0x0200_0000;
const P_LATIN_OTHER: u32 = 0x0210_0000;
const P_KANA: u32 = 0x0300_0000;
const P_KANJI_JIS: u32 = 0x0400_0000;
const P_HAN_CORE: u32 = 0x0500_0000;
const P_HAN_EXT: u32 = 0x0600_0000;
const P_OTHER: u32 = 0x0800_0000;

// Secondary weights.
const S_BASE: u16 = 0x21;
const S_DERIVED: u16 = 0x20;
const S_ACCENT: u16 = 0x40;

// Tertiary weights.
const T_LOWER: u16 = 0x02;
const T_KANA_SMALL: u16 = 0x02;
const T_KANA: u16 = 0x03;
const T_COMPAT: u16 = 0x04;
const T_SUPERSCRIPT: u16 = 0x05;
const T_CIRCLED: u16 = 0x06;
const T_UPPER: u16 = 0x08;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Element {
    primary: u32,
    secondary: u16,
    tertiary: u16,
}

const fn el(primary: u32, secondary: u16, tertiary: u16) -> Element {
    Element {
        primary,
        secondary,
        tertiary,
    }
}

/// A kana syllable: base = unvoiced, normal-size hiragana code point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kana {
    base: u32,
    voicing: u8,
    small: bool,
}

fn kana_element(k: Kana) -> Element {
    el(
        P_KANA + k.base,
        S_BASE + 2 * u16::from(k.voicing),
        if k.small { T_KANA_SMALL } else { T_KANA },
    )
}

/// Half-width katakana U+FF66..=U+FF9D → full-width katakana (0 = `ｰ`, handled separately).
const HALFWIDTH_KATAKANA: [u16; 56] = [
    0x30F2, 0x30A1, 0x30A3, 0x30A5, 0x30A7, 0x30A9, 0x30E3, 0x30E5, 0x30E7, 0x30C3, 0, 0x30A2,
    0x30A4, 0x30A6, 0x30A8, 0x30AA, 0x30AB, 0x30AD, 0x30AF, 0x30B1, 0x30B3, 0x30B5, 0x30B7, 0x30B9,
    0x30BB, 0x30BD, 0x30BF, 0x30C1, 0x30C4, 0x30C6, 0x30C8, 0x30CA, 0x30CB, 0x30CC, 0x30CD, 0x30CE,
    0x30CF, 0x30D2, 0x30D5, 0x30D8, 0x30DB, 0x30DE, 0x30DF, 0x30E0, 0x30E1, 0x30E2, 0x30E4, 0x30E6,
    0x30E8, 0x30E9, 0x30EA, 0x30EB, 0x30EC, 0x30ED, 0x30EF, 0x30F3,
];

fn kana(cp: u32) -> Option<Kana> {
    let cp = match cp {
        0xFF66..=0xFF9D => match HALFWIDTH_KATAKANA[(cp - 0xFF66) as usize] {
            0 => return None,
            full => u32::from(full),
        },
        _ => cp,
    };
    let voiced = |base: u32| Kana {
        base,
        voicing: 1,
        small: false,
    };
    let hira = match cp {
        0x30A1..=0x30F6 => cp - 0x60,
        0x30F7 => return Some(voiced(0x308F)), // ヷ
        0x30F8 => return Some(voiced(0x3090)), // ヸ
        0x30F9 => return Some(voiced(0x3091)), // ヹ
        0x30FA => return Some(voiced(0x3092)), // ヺ
        0x3041..=0x3096 => cp,
        _ => return None,
    };
    let (base, voicing, small) = match hira {
        0x3041 | 0x3043 | 0x3045 | 0x3047 | 0x3049 => (hira + 1, 0, true),
        0x304B..=0x3062 => {
            let voicing = ((hira - 0x304B) % 2) as u8;
            (hira - u32::from(voicing), voicing, false)
        }
        0x3063 => (0x3064, 0, true),
        0x3065 | 0x3067 | 0x3069 => (hira - 1, 1, false),
        0x306F..=0x307D => {
            let voicing = ((hira - 0x306F) % 3) as u8;
            (hira - u32::from(voicing), voicing, false)
        }
        0x3083 | 0x3085 | 0x3087 => (hira + 1, 0, true),
        0x308E => (0x308F, 0, true),
        0x3094 => (0x3046, 1, false),
        0x3095 => (0x304B, 0, true),
        0x3096 => (0x3051, 0, true),
        _ => (hira, 0, false),
    };
    Some(Kana {
        base,
        voicing,
        small,
    })
}

/// Vowel syllable that `ー` takes after a syllable (`ん` keeps `ん`).
fn vowel_of(base: u32) -> u32 {
    const A: u32 = 0x3042;
    const I: u32 = 0x3044;
    const U: u32 = 0x3046;
    const E: u32 = 0x3048;
    const O: u32 = 0x304A;
    match base {
        0x3042 | 0x304B | 0x3055 | 0x305F | 0x306A | 0x306F | 0x307E | 0x3084 | 0x3089 | 0x308F => {
            A
        }
        0x3044 | 0x304D | 0x3057 | 0x3061 | 0x306B | 0x3072 | 0x307F | 0x308A | 0x3090 => I,
        0x3046 | 0x304F | 0x3059 | 0x3064 | 0x306C | 0x3075 | 0x3080 | 0x3086 | 0x308B => U,
        0x3048 | 0x3051 | 0x305B | 0x3066 | 0x306D | 0x3078 | 0x3081 | 0x308C | 0x3091 => E,
        0x304A | 0x3053 | 0x305D | 0x3068 | 0x306E | 0x307B | 0x3082 | 0x3088 | 0x308D | 0x3092 => {
            O
        }
        other => other,
    }
}

fn whitespace_rank(cp: u32) -> Option<u32> {
    Some(match cp {
        0x09 => 1,
        0x0A => 2,
        0x0B => 3,
        0x0C => 4,
        0x0D => 5,
        0x85 => 6,
        0x2028 => 7,
        0x2029 => 8,
        0x20 | 0xA0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000 => 9,
        _ => return None,
    })
}

fn is_ignorable(cp: u32) -> bool {
    matches!(
        cp,
        0x00..=0x08
            | 0x0E..=0x1F
            | 0x7F..=0x84
            | 0x86..=0x9F
            | 0xAD
            | 0x200B..=0x200F
            | 0x2060..=0x2064
            | 0xFE00..=0xFE0F
            | 0xFEFF
            | 0xE0100..=0xE01EF
    )
}

fn jis_index(c: char) -> Option<u32> {
    static TABLE: OnceLock<Vec<(char, u32)>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table: Vec<(char, u32)> = Vec::with_capacity(JIS_KANJI_COUNT);
        for (index, kanji) in JIS_KANJI.chars().enumerate() {
            table.push((kanji, u32::try_from(index).unwrap_or(u32::MAX)));
        }
        table.sort_unstable_by_key(|&(kanji, _)| kanji);
        table
    });
    table
        .binary_search_by_key(&c, |&(kanji, _)| kanji)
        .ok()
        .map(|i| table[i].1)
}

fn symbol_rank(cp: u32) -> Option<u32> {
    SYMBOL_RANKS
        .binary_search_by_key(&cp, |&(code, _)| code)
        .ok()
        .map(|i| u32::from(SYMBOL_RANKS[i].1))
}

// Accent ranks (secondary order of the common Latin diacritics in the UCA).
const ACUTE: u16 = 1;
const GRAVE: u16 = 2;
const CIRCUMFLEX: u16 = 4;
const RING: u16 = 6;
const DIAERESIS: u16 = 7;
const TILDE: u16 = 9;
const STROKE: u16 = 11;
const CEDILLA: u16 = 12;

fn latin_letter(lower: u8, upper: bool) -> Element {
    el(
        P_LATIN + u32::from(lower - b'a') * 4,
        S_BASE,
        if upper { T_UPPER } else { T_LOWER },
    )
}

/// Latin-1 Supplement letters: pushes their elements, returns false for other characters.
fn push_latin1(cp: u32, out: &mut Vec<Element>) -> bool {
    let upper = (0xC0..=0xDE).contains(&cp);
    let lower_cp = if upper { cp + 0x20 } else { cp };
    let case = if upper { T_UPPER } else { T_LOWER };
    let (base, accent) = match lower_cp {
        0xE0 => (b'a', GRAVE),
        0xE1 => (b'a', ACUTE),
        0xE2 => (b'a', CIRCUMFLEX),
        0xE3 => (b'a', TILDE),
        0xE4 => (b'a', DIAERESIS),
        0xE5 => (b'a', RING),
        0xE7 => (b'c', CEDILLA),
        0xE8 => (b'e', GRAVE),
        0xE9 => (b'e', ACUTE),
        0xEA => (b'e', CIRCUMFLEX),
        0xEB => (b'e', DIAERESIS),
        0xEC => (b'i', GRAVE),
        0xED => (b'i', ACUTE),
        0xEE => (b'i', CIRCUMFLEX),
        0xEF => (b'i', DIAERESIS),
        0xF1 => (b'n', TILDE),
        0xF2 => (b'o', GRAVE),
        0xF3 => (b'o', ACUTE),
        0xF4 => (b'o', CIRCUMFLEX),
        0xF5 => (b'o', TILDE),
        0xF6 => (b'o', DIAERESIS),
        0xF8 => (b'o', STROKE),
        0xF9 => (b'u', GRAVE),
        0xFA => (b'u', ACUTE),
        0xFB => (b'u', CIRCUMFLEX),
        0xFC => (b'u', DIAERESIS),
        0xFD | 0xFF => (b'y', if lower_cp == 0xFD { ACUTE } else { DIAERESIS }),
        0xE6 => {
            // æ / Æ: "ae" with a tertiary difference.
            out.push(el(
                latin_letter(b'a', false).primary,
                S_BASE,
                T_COMPAT + case,
            ));
            out.push(el(
                latin_letter(b'e', false).primary,
                S_BASE,
                T_COMPAT + case,
            ));
            return true;
        }
        0xDF => {
            // ß: "ss" with a tertiary difference.
            out.push(el(latin_letter(b's', false).primary, S_BASE, T_COMPAT));
            out.push(el(latin_letter(b's', false).primary, S_BASE, T_COMPAT));
            return true;
        }
        0xF0 => {
            // ð / Ð: own letter after d.
            out.push(el(latin_letter(b'd', false).primary + 2, S_BASE, case));
            return true;
        }
        0xFE => {
            // þ / Þ: own letter after z.
            out.push(el(latin_letter(b'z', false).primary + 2, S_BASE, case));
            return true;
        }
        _ => return false,
    };
    out.push(latin_letter(base, upper));
    out.push(el(0, S_ACCENT + accent, T_LOWER));
    true
}

fn single_lowercase(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

fn case_tertiary(c: char, lower: char) -> u16 {
    if lower == c {
        T_LOWER
    } else {
        T_UPPER
    }
}

fn elements(text: &str) -> Vec<Element> {
    let mut out: Vec<Element> = Vec::with_capacity(text.len());
    let mut prev_kana: Option<Kana> = None;
    for c in text.chars() {
        let mut cp = u32::from(c);
        // Marks that depend on the preceding kana.
        match cp {
            0x30FC => {
                if let Some(k) = prev_kana.take() {
                    out.push(el(P_KANA + vowel_of(k.base), S_DERIVED, T_KANA));
                    continue;
                }
            }
            0x309D | 0x309E | 0x30FD | 0x30FE => {
                if let Some(k) = prev_kana.take() {
                    let voiced = matches!(cp, 0x309E | 0x30FE);
                    let secondary = S_DERIVED + if voiced { 2 } else { 0 };
                    out.push(el(P_KANA + k.base, secondary, T_KANA));
                    continue;
                }
            }
            0x3099 | 0xFF9E | 0x309A | 0xFF9F => {
                let voicing = if matches!(cp, 0x3099 | 0xFF9E) { 1 } else { 2 };
                match out.last_mut() {
                    Some(last) if prev_kana.is_some() && last.secondary == S_BASE => {
                        last.secondary = S_BASE + 2 * voicing;
                    }
                    _ => out.push(el(0, S_ACCENT + 20 + voicing, T_LOWER)),
                }
                continue;
            }
            _ => {}
        }
        if is_ignorable(cp) {
            continue;
        }
        prev_kana = None;
        if let Some(rank) = whitespace_rank(cp) {
            out.push(el(P_SPACE + rank, S_BASE, T_LOWER));
            continue;
        }
        // Full-width ASCII compares like ASCII.
        if (0xFF01..=0xFF5E).contains(&cp) {
            cp -= 0xFEE0;
        }
        if let Some(k) = kana(cp) {
            out.push(kana_element(k));
            prev_kana = Some(k);
            continue;
        }
        if cp == 0x309F || cp == 0x30FF {
            // ゟ (より) / ヿ (こと) digraphs.
            let pair: [u32; 2] = if cp == 0x309F {
                [0x3088, 0x308A]
            } else {
                [0x3053, 0x3068]
            };
            for part in pair {
                if let Some(k) = kana(part) {
                    out.push(kana_element(k));
                    prev_kana = Some(k);
                }
            }
            continue;
        }
        match cp {
            0x30..=0x39 => out.push(el(P_DIGIT + (cp - 0x30), S_BASE, T_LOWER)),
            0x41..=0x5A => out.push(latin_letter((cp + 0x20) as u8, true)),
            0x61..=0x7A => out.push(latin_letter(cp as u8, false)),
            0xB9 => out.push(el(P_DIGIT + 1, S_BASE, T_SUPERSCRIPT)),
            0xB2 | 0xB3 => out.push(el(P_DIGIT + (cp - 0xB0), S_BASE, T_SUPERSCRIPT)),
            0x2460..=0x2468 => out.push(el(P_DIGIT + (cp - 0x245F), S_BASE, T_CIRCLED)),
            0x0152 | 0x0153 => {
                let case = if cp == 0x0152 { T_UPPER } else { T_LOWER };
                out.push(el(
                    latin_letter(b'o', false).primary,
                    S_BASE,
                    T_COMPAT + case,
                ));
                out.push(el(
                    latin_letter(b'e', false).primary,
                    S_BASE,
                    T_COMPAT + case,
                ));
            }
            0xC0..=0xFF if push_latin1(cp, &mut out) => {}
            _ => {
                let ch = char::from_u32(cp).unwrap_or(c);
                if let Some(index) = jis_index(ch) {
                    out.push(el(P_KANJI_JIS + index, S_BASE, T_LOWER));
                } else if matches!(cp, 0x4E00..=0x9FFF | 0xF900..=0xFAFF) {
                    out.push(el(P_HAN_CORE + cp, S_BASE, T_LOWER));
                } else if matches!(cp, 0x3400..=0x4DBF | 0x20000..=0x3FFFF) {
                    out.push(el(P_HAN_EXT + cp, S_BASE, T_LOWER));
                } else if let Some(rank) = symbol_rank(cp) {
                    out.push(el(P_SYMBOL_RANKED + rank, S_BASE, T_LOWER));
                } else if ch.is_alphabetic() {
                    let lower = single_lowercase(ch);
                    let group = if matches!(cp, 0x0100..=0x024F | 0x1E00..=0x1EFF) {
                        P_LATIN_OTHER
                    } else {
                        P_OTHER
                    };
                    out.push(el(
                        group + u32::from(lower),
                        S_BASE,
                        case_tertiary(ch, lower),
                    ));
                } else if ch.is_numeric() {
                    out.push(el(P_NUMERIC_OTHER + cp, S_BASE, T_LOWER));
                } else {
                    out.push(el(P_SYMBOL_OTHER + cp, S_BASE, T_LOWER));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(items: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = items.iter().map(|s| (*s).to_owned()).collect();
        v.sort_by(|a, b| ja_compare(a, b));
        v
    }

    /// Golden: spec-lib-core §7.5 (verified ICU order).
    #[test]
    fn spec_sample_order() {
        let input = [
            "名前は岡部",
            "猫を飼っている",
            "東京在住",
            "コーヒー派",
            "ゲームが好き",
            "アニメが好き",
            "Dr.Pepperが好き",
            "ABC",
            "abc",
            "10代",
        ];
        assert_eq!(
            sorted(&input),
            [
                "10代",
                "abc",
                "ABC",
                "Dr.Pepperが好き",
                "アニメが好き",
                "ゲームが好き",
                "コーヒー派",
                "東京在住",
                "猫を飼っている",
                "名前は岡部"
            ]
        );
    }

    #[test]
    fn jis_table_is_complete() {
        assert_eq!(JIS_KANJI.chars().count(), JIS_KANJI_COUNT);
        assert_eq!(jis_index('亜'), Some(0));
        assert_eq!(jis_index('熙'), Some(6354));
        assert_eq!(jis_index('あ'), None);
        assert!(SYMBOL_RANKS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    /// Golden: Node 22 `a.localeCompare(b, "ja")` (sign).
    #[test]
    fn pairwise_matches_icu() {
        use Ordering::{Equal as E, Greater as G, Less as L};
        let cases: [(&str, &str, Ordering); 52] = [
            ("キー", "きあ", G),
            ("キー", "きい", L),
            ("カー", "かあ", L),
            ("カー", "かぁ", L),
            ("かあ", "かぁ", G),
            ("か", "が", L),
            ("かんじ", "がっこう", G),
            ("は", "ば", L),
            ("ば", "ぱ", L),
            ("あ", "ア", E),
            ("ア", "ｱ", E),
            ("ぁ", "あ", L),
            ("ァ", "あ", L),
            ("abc", "ABC", L),
            ("ABC", "ａｂｃ", G),
            ("abc", "ａｂｃ", E),
            ("1", "１", E),
            ("a", "1", G),
            ("!", "1", L),
            (" ", "!", L),
            ("。", "a", L),
            ("ー", "あ", L),
            ("ーあ", "あ", L),
            ("々", "あ", L),
            ("あ", "一", L),
            ("いゝ", "いい", L),
            ("いゞ", "いい", G),
            ("きゞ", "きぎ", L),
            ("カード", "かーど", E),
            ("ｶｰﾄﾞ", "カード", L),
            ("é", "e", G),
            ("é", "f", L),
            ("ü", "ű", L),
            ("Ω", "z", G),
            ("Ω", "あ", G),
            ("Ω", "𠮷", G),
            ("猫", "名", L),
            ("亜", "一", L),
            ("𠮷", "亜", G),
            ("丼", "猫", G),
            ("ゔ", "う", G),
            ("ヴ", "ゔ", E),
            ("ん", "ゔ", G),
            ("ゖ", "け", L),
            ("ゟ", "よ", G),
            ("がー", "があ", L),
            ("がー", "かあ", G),
            ("ゃー", "やあ", L),
            ("っー", "つう", L),
            ("a b", "ab", L),
            ("Ab", "aB", G),
            ("日本", "日 本", G),
        ];
        for (a, b, expected) in cases {
            assert_eq!(ja_compare(a, b), expected, "{a:?} vs {b:?}");
            assert_eq!(ja_compare(b, a), expected.reverse(), "{b:?} vs {a:?}");
        }
    }
}
