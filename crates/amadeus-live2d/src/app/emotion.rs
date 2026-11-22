//! `ReamaCubismPlayer` data tables (pipeline spec §9.1, §9.3, §9.10), copied verbatim from the
//! deployed bundle (`EMOTION_TARGETS`, `MOTION_BURSTS`, the vowel strings of
//! `getVowelMouthOpening`). Pure.

/// One emotion's face/pose target (`EMOTION_TARGETS[tag]`). Field order is the JS key order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmotionTarget {
    pub brow_y: f64,
    pub brow_form: f64,
    pub brow_angle: f64,
    pub eye_open: f64,
    pub eye_smile: f64,
    pub mouth_form: f64,
    pub body_angle_x: f64,
    pub body_angle_y: f64,
    pub body_angle_z: f64,
    pub head_angle_x: f64,
    pub head_angle_y: f64,
    pub head_angle_z: f64,
    pub cheek: f64,
    /// Only `WINK` has `isWink: true` (right eye forced shut).
    pub is_wink: bool,
}

impl EmotionTarget {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        brow_y: f64,
        brow_form: f64,
        brow_angle: f64,
        eye_open: f64,
        eye_smile: f64,
        mouth_form: f64,
        body_angle_x: f64,
        body_angle_y: f64,
        body_angle_z: f64,
        head_angle_x: f64,
        head_angle_y: f64,
        head_angle_z: f64,
        cheek: f64,
    ) -> Self {
        Self {
            brow_y,
            brow_form,
            brow_angle,
            eye_open,
            eye_smile,
            mouth_form,
            body_angle_x,
            body_angle_y,
            body_angle_z,
            head_angle_x,
            head_angle_y,
            head_angle_z,
            cheek,
            is_wink: false,
        }
    }

    const fn wink(mut self) -> Self {
        self.is_wink = true;
        self
    }

    /// The 13 numeric fields blended with `lerp(self.f, to.f, amount)`; `is_wink` is dropped
    /// (V1 builds a fresh object literal without it — the wake-up QUIRK of spec §9.7 5c).
    pub fn lerp_to(&self, to: &EmotionTarget, amount: f64) -> EmotionTarget {
        use super::noise::lerp;
        EmotionTarget {
            brow_y: lerp(self.brow_y, to.brow_y, amount),
            brow_form: lerp(self.brow_form, to.brow_form, amount),
            brow_angle: lerp(self.brow_angle, to.brow_angle, amount),
            eye_open: lerp(self.eye_open, to.eye_open, amount),
            eye_smile: lerp(self.eye_smile, to.eye_smile, amount),
            mouth_form: lerp(self.mouth_form, to.mouth_form, amount),
            body_angle_x: lerp(self.body_angle_x, to.body_angle_x, amount),
            body_angle_y: lerp(self.body_angle_y, to.body_angle_y, amount),
            body_angle_z: lerp(self.body_angle_z, to.body_angle_z, amount),
            head_angle_x: lerp(self.head_angle_x, to.head_angle_x, amount),
            head_angle_y: lerp(self.head_angle_y, to.head_angle_y, amount),
            head_angle_z: lerp(self.head_angle_z, to.head_angle_z, amount),
            cheek: lerp(self.cheek, to.cheek, amount),
            is_wink: false,
        }
    }
}

/// `EMOTION_TARGETS` in JS key order.
pub static EMOTION_TARGETS: [(&str, EmotionTarget); 12] = [
    (
        "NORMAL",
        EmotionTarget::new(
            0.0, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ),
    ),
    (
        "SMILE",
        EmotionTarget::new(
            0.4, 0.8, 0.2, 0.6, 1.0, 1.0, 3.0, 0.0, -2.0, 5.0, 3.0, -3.0, 0.3,
        ),
    ),
    (
        "ANGRY",
        EmotionTarget::new(
            -0.6, -0.8, -0.8, 0.5, 0.0, -0.8, -4.0, 0.0, 0.0, -5.0, -5.0, 0.0, 0.0,
        ),
    ),
    (
        "SAD",
        EmotionTarget::new(
            -0.5, -0.6, 0.6, 0.4, 0.0, -0.5, -3.0, -3.0, -3.0, -8.0, -7.0, -5.0, 0.0,
        ),
    ),
    (
        "SURPRISED",
        EmotionTarget::new(
            0.8, 0.5, 0.0, 1.25, 0.0, -0.4, -2.0, 5.0, 2.0, 2.0, 8.0, 0.0, 0.0,
        ),
    ),
    (
        "BLUSH",
        EmotionTarget::new(
            0.2, 0.4, 0.3, 0.6, 0.7, 0.3, 8.0, -3.0, -3.0, 8.0, -8.0, -7.0, 1.0,
        ),
    ),
    (
        "WINK",
        EmotionTarget::new(
            0.5, 0.8, 0.2, 1.0, 0.7, 0.5, 2.0, 0.0, -5.0, 5.0, 2.0, -5.0, 0.0,
        )
        .wink(),
    ),
    (
        "DISGUST",
        EmotionTarget::new(
            -0.3, -0.9, -0.5, 0.4, 0.0, -0.9, 5.0, 4.0, 2.0, 5.0, -4.0, 3.0, 0.0,
        ),
    ),
    (
        "SMUG",
        EmotionTarget::new(
            0.3, 0.7, 0.4, 0.6, 0.8, 0.6, 4.0, 0.0, -2.0, 10.0, 5.0, -2.0, 0.0,
        ),
    ),
    (
        "THINKING",
        EmotionTarget::new(
            -0.2, -0.3, 0.2, 0.8, 0.0, -0.2, -2.0, 5.0, 5.0, 5.0, -8.0, 5.0, 0.0,
        ),
    ),
    (
        "PANIC",
        EmotionTarget::new(
            0.6, -0.5, -0.3, 1.2, 0.0, -0.5, -3.0, 0.0, 0.0, -2.0, 0.0, 0.0, 0.6,
        ),
    ),
    (
        "SLEEPING",
        EmotionTarget::new(
            -0.3, 0.2, -0.2, 0.0, 0.0, -0.1, -1.0, -2.0, -2.0, -12.0, -10.0, -8.0, 0.0,
        ),
    ),
];

/// `EMOTION_TARGETS[tag]` (exact, case-sensitive key match; `None` = JS `undefined`).
pub fn emotion_target(tag: &str) -> Option<&'static EmotionTarget> {
    EMOTION_TARGETS
        .iter()
        .find(|(key, _)| *key == tag)
        .map(|(_, target)| target)
}

/// `EMOTION_TARGETS.NORMAL`.
pub fn normal_target() -> &'static EmotionTarget {
    &EMOTION_TARGETS[0].1
}

/// `EMOTION_TARGETS.SURPRISED`.
pub fn surprised_target() -> &'static EmotionTarget {
    &EMOTION_TARGETS[4].1
}

/// One `MOTION_BURSTS` entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionBurst {
    pub body_x: f64,
    pub body_y: f64,
    pub body_z: f64,
    pub head_x: f64,
    pub head_y: f64,
    pub head_z: f64,
    /// Seconds.
    pub duration: f64,
    pub intensity: f64,
}

impl MotionBurst {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        body_x: f64,
        body_y: f64,
        body_z: f64,
        head_x: f64,
        head_y: f64,
        head_z: f64,
        duration: f64,
        intensity: f64,
    ) -> Self {
        Self {
            body_x,
            body_y,
            body_z,
            head_x,
            head_y,
            head_z,
            duration,
            intensity,
        }
    }
}

/// `MOTION_BURSTS` in JS key order (SLEEPING and SNEEZE are only used for their duration).
pub static MOTION_BURSTS: [(&str, MotionBurst); 13] = [
    (
        "NORMAL",
        MotionBurst::new(0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.4, 0.5),
    ),
    (
        "SMILE",
        MotionBurst::new(4.0, 2.0, -2.0, 5.0, 3.0, -3.0, 0.5, 0.8),
    ),
    (
        "ANGRY",
        MotionBurst::new(-5.0, -3.0, 0.0, -5.0, -5.0, 0.0, 0.5, 1.2),
    ),
    (
        "SAD",
        MotionBurst::new(-2.0, -3.0, -2.0, -3.0, -4.0, -2.0, 0.8, 0.7),
    ),
    (
        "SURPRISED",
        MotionBurst::new(-2.0, 5.0, 2.0, 0.0, 8.0, 0.0, 0.4, 1.5),
    ),
    (
        "BLUSH",
        MotionBurst::new(8.0, -2.0, -3.0, 12.0, -3.0, -5.0, 0.6, 1.0),
    ),
    (
        "WINK",
        MotionBurst::new(5.0, 2.0, -2.0, 6.0, 3.0, -4.0, 0.4, 0.9),
    ),
    (
        "DISGUST",
        MotionBurst::new(5.0, 4.0, 2.0, 7.0, -3.0, 3.0, 0.5, 1.1),
    ),
    (
        "SMUG",
        MotionBurst::new(2.0, 2.0, -1.0, 5.0, 3.0, -1.0, 0.7, 0.6),
    ),
    (
        "THINKING",
        MotionBurst::new(0.0, 0.0, 0.0, 2.0, -3.0, 1.0, 0.8, 0.3),
    ),
    (
        "PANIC",
        MotionBurst::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.2, 2.0),
    ),
    (
        "SLEEPING",
        MotionBurst::new(-1.0, -2.0, -1.0, -2.0, -3.0, -2.0, 4.5, 0.3),
    ),
    (
        "SNEEZE",
        MotionBurst::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.2, 1.0),
    ),
];

/// `MOTION_BURSTS[tag] || MOTION_BURSTS.NORMAL` (unknown tags play the NORMAL burst).
pub fn motion_burst(tag: &str) -> &'static MotionBurst {
    MOTION_BURSTS
        .iter()
        .find(|(key, _)| *key == tag)
        .map_or(&MOTION_BURSTS[0].1, |(_, burst)| burst)
}

/// `aVowels` (verbatim; note the small kana listed twice and the missing full-size ア — a V1 typo
/// that is reproduced, spec §9.10).
pub const A_VOWELS: &str = "あかさたなはまやらわがざだばぱぁゃァカサタナハマヤラワガザダバパァャ";
/// `oVowels` (verbatim).
pub const O_VOWELS: &str = "おこそとのほもよろをごぞどぼぽぉょォコソトノホモヨロヲゴゾドボポォョ";
/// `eVowels` (verbatim).
pub const E_VOWELS: &str = "えけせてねへめれげぜでべぺぇェケセテネヘメレゲゼデベペェ";
/// `uVowels` (verbatim).
pub const U_VOWELS: &str = "うくすつぬふむゆるぐずづぶぷぅゅゥクスツヌフムユルグズヅブプゥュ";
/// `iVowels` (verbatim).
pub const I_VOWELS: &str = "いきしちにひみりぎじぢびぴぃィキシチニヒミリギジヂビピィ";
/// `closedChars` (verbatim, with the JS escapes `\n \r \t` decoded).
pub const CLOSED_CHARS: &str = "んン。、？！.,?! \u{3000}\n\r\t-_";

/// `getVowelMouthOpening(chr)`: mouth opening for the first character of `chr` (`charAt(0)`;
/// no table entry is outside the BMP, so the first `char` is equivalent to the first UTF-16 unit).
pub fn vowel_mouth_opening(chr: &str) -> f64 {
    let Some(c) = chr.chars().next() else {
        return 0.0;
    };
    match c {
        'a' | 'A' => return 0.9,
        'o' | 'O' => return 0.7,
        'e' | 'E' => return 0.6,
        'u' | 'U' => return 0.4,
        'i' | 'I' | 'í' | 'Í' => return 0.2,
        _ => {}
    }
    if A_VOWELS.contains(c) {
        0.9
    } else if O_VOWELS.contains(c) {
        0.7
    } else if E_VOWELS.contains(c) {
        0.6
    } else if U_VOWELS.contains(c) {
        0.4
    } else if I_VOWELS.contains(c) {
        0.2
    } else if CLOSED_CHARS.contains(c) {
        0.0
    } else {
        0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16_len(s: &str) -> usize {
        s.encode_utf16().count()
    }

    #[test]
    fn table_lengths_match_the_bundle() {
        assert_eq!(utf16_len(A_VOWELS), 34);
        assert_eq!(utf16_len(O_VOWELS), 34);
        assert_eq!(utf16_len(E_VOWELS), 28);
        assert_eq!(utf16_len(U_VOWELS), 32);
        assert_eq!(utf16_len(I_VOWELS), 28);
        assert_eq!(utf16_len(CLOSED_CHARS), 17);
        // First / last code points of the spec's U+ lists.
        assert_eq!(A_VOWELS.chars().next(), Some('\u{3042}'));
        assert_eq!(A_VOWELS.chars().last(), Some('\u{30E3}'));
        assert_eq!(O_VOWELS.chars().nth(25), Some('\u{30ED}'));
        assert_eq!(U_VOWELS.chars().last(), Some('\u{30E5}'));
        assert_eq!(I_VOWELS.chars().nth(14), Some('\u{30A3}'));
    }

    /// Spec §12.1 `getVowelMouthOpening` vectors (produced by the deployed bundle).
    #[test]
    fn vowel_vectors() {
        let cases: &[(&str, f64)] = &[
            ("a", 0.9),
            ("A", 0.9),
            ("o", 0.7),
            ("O", 0.7),
            ("e", 0.6),
            ("E", 0.6),
            ("u", 0.4),
            ("U", 0.4),
            ("i", 0.2),
            ("I", 0.2),
            ("í", 0.2),
            ("Í", 0.2),
            ("é", 0.5),
            ("x", 0.5),
            ("1", 0.5),
            ("あ", 0.9),
            ("か", 0.9),
            ("ア", 0.5),
            ("カ", 0.9),
            ("ぁ", 0.9),
            ("ゃ", 0.9),
            ("ャ", 0.9),
            ("お", 0.7),
            ("オ", 0.5),
            ("ょ", 0.7),
            ("ョ", 0.7),
            ("を", 0.7),
            ("ヲ", 0.7),
            ("え", 0.6),
            ("エ", 0.5),
            ("う", 0.4),
            ("ウ", 0.5),
            ("ゅ", 0.4),
            ("ュ", 0.4),
            ("い", 0.2),
            ("イ", 0.5),
            ("ん", 0.0),
            ("ン", 0.0),
            ("っ", 0.5),
            ("ッ", 0.5),
            ("ー", 0.5),
            ("ヴ", 0.5),
            ("ゎ", 0.5),
            ("ゐ", 0.5),
            ("ゑ", 0.5),
            ("。", 0.0),
            ("、", 0.0),
            ("？", 0.0),
            ("！", 0.0),
            (".", 0.0),
            (",", 0.0),
            ("?", 0.0),
            ("!", 0.0),
            (" ", 0.0),
            ("\u{3000}", 0.0),
            ("\n", 0.0),
            ("\r", 0.0),
            ("\t", 0.0),
            ("-", 0.0),
            ("_", 0.0),
            ("紅", 0.5),
            ("😀", 0.5),
            ("ab", 0.9),
            ("おはよう", 0.7),
            (" x", 0.0),
            ("", 0.0),
        ];
        for (chr, expected) in cases {
            assert_eq!(vowel_mouth_opening(chr), *expected, "{chr:?}");
        }
    }

    #[test]
    fn lookups() {
        assert_eq!(emotion_target("SMILE").map(|t| t.cheek), Some(0.3));
        assert!(emotion_target("smile").is_none());
        assert!(emotion_target("SNEEZE").is_none());
        assert!(emotion_target("WINK").is_some_and(|t| t.is_wink));
        assert_eq!(EMOTION_TARGETS.iter().filter(|(_, t)| t.is_wink).count(), 1);
        assert_eq!(motion_burst("SNEEZE").duration, 2.2);
        assert_eq!(motion_burst("FOO"), motion_burst("NORMAL"));
        assert_eq!(motion_burst("").duration, 0.4);
        assert_eq!(surprised_target().eye_open, 1.25);
        assert_eq!(normal_target().brow_form, 0.5);
    }
}
