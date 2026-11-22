//! The desktop boot log (V1 `BOOT_LOG` + the `LoadingPanel` sequence) and the change log
//! (V1 `CHANGE_LOG_DATA`, with a V2 entry on top), `RealAmadeusApp.tsx` 265–323 and 2063–2187.
//!
//! `BOOT_LOG` was generated mechanically from the V1 source and is pinned by SHA-256 golden
//! tests. Its spelling (`Initakising`, `/sts`, `ROSS:`, double spaces) is in-universe lore and
//! must stay verbatim, as must the header `Amadeus System Ver 1.09.2 rev.2123` (not the app
//! version).

use crate::jscompat::{js_trim, utf16_len};
use crate::settings::jsnum::js_parse_int;

/// V1 `BOOT_LOG` (41 lines, pure ASCII). `[MEM]` (once) is replaced by the animated memory
/// value; `<pos=N%>` tags are turned into padding by [`preprocess_boot_log`].
pub const BOOT_LOG: &str = concat!(
    "Amadeus System Ver 1.09.2 rev.2123\n",
    ">>Initialize System ...  OK\n",
    ">>Detecting boot device ... OK\n",
    ">>Loading Kernel ...  OK\n",
    ">>Detecting OS control device ...  OK\n",
    ">>Booting ...\n",
    ">>Processor 0 is Activate ...  OK\n",
    ">>Processor 1 is Activate ...  OK\n",
    ">>Processor 2 is Activate ...  OK\n",
    ">>Processor 3 is Activate ...  OK\n",
    ">>Memory Initialize [MEM]/32767MBytes\n\n\n\n\n",
    "INIT: Kernel version 2.04 booting...\n\n\n\n\n",
    "ROSS:\n\n\n\n\n",
    "Mounting proc at /proc...<pos=30%>[OK]\n",
    "Mounting sysfs at /sts...<pos=30%>[OK]\n",
    "Initakising network<pos=30%>[OK]\n",
    "Setting up localhost ...<pos=30%>[OK]\n",
    "Setting up inet1 ...<pos=30%>[OK]\n",
    "Setting up route ...<pos=30%>[OK]\n",
    "Accessing Cloud ...<pos=30%>[OK]\n",
    "Starting system log at /log/sys...<pos=30%>[OK]\n",
    "Cleaning /var/lock<pos=30%>[OK]\n",
    "Cleaning /tmp<pos=30%>[OK]\n",
    "Updating init.rc<pos=30%>[OK]\n\n\n\n\n",
    "Boot Sequences Start...",
);

/// Placeholder of the memory counter in [`BOOT_LOG`].
pub const BOOT_LOG_MEM_PLACEHOLDER: &str = "[MEM]";
/// Final value of the animated memory counter (MBytes).
pub const BOOT_LOG_MEM_MAX: u32 = 32767;
/// Canvas font V1 measures `"M"` with to estimate the character width.
pub const BOOT_FONT: &str = "18px \"DepartureMono\", monospace";
/// Character width used when measuring is impossible or yields 0.
pub const BOOT_CHAR_WIDTH_FALLBACK: f64 = 10.0;
/// Width of the virtual stage the `<pos=N%>` tags refer to.
pub const BOOT_STAGE_WIDTH: f64 = 1920.0;
/// Defensive cap on the padding of one `<pos>` tag (V1 would throw a `RangeError` for an
/// absurd width instead; realistic widths need < 200 spaces).
const MAX_BOOT_PADDING: usize = 65_536;
/// Delay before the boot text appears.
pub const BOOT_INITIAL_DELAY_MS: u32 = 500;
/// Duration of the memory counter animation.
pub const BOOT_MEMORY_ANIMATION_MS: u32 = 1500;
/// Frame step of the memory counter animation.
pub const BOOT_MEMORY_STEP_MS: u32 = 30;
/// Delay after a line containing `...` or a blank line.
pub const BOOT_SLOW_LINE_DELAY_MS: u32 = 400;
/// Delay after any other line.
pub const BOOT_FAST_LINE_DELAY_MS: u32 = 50;
/// Delay between the last line and the logo (`amadeus_logo_v3.png`).
pub const BOOT_LOGO_DELAY_MS: u32 = 2000;
/// Delay between the logo and `onFinished()`.
pub const BOOT_FINISH_DELAY_MS: u32 = 4000;

/// `BOOT_LOG.replace("[MEM]", String(memory))` (first occurrence only).
pub fn boot_log_with_memory(memory: u32) -> String {
    BOOT_LOG.replacen(BOOT_LOG_MEM_PLACEHOLDER, &memory.to_string(), 1)
}

/// V1 `estimateCharWidth()`: `measureText("M").width || 10`, 10 without a 2D context.
pub fn normalize_boot_char_width(measured: Option<f64>) -> f64 {
    match measured {
        Some(w) if w != 0.0 && !w.is_nan() => w,
        _ => BOOT_CHAR_WIDTH_FALLBACK,
    }
}

/// First `<pos=(\d+)%>` match of a line: `(start, end, percent)`.
fn find_pos_tag(line: &str) -> Option<(usize, usize, f64)> {
    let mut from = 0;
    while let Some(offset) = line[from..].find("<pos=") {
        let start = from + offset;
        let digits_start = start + "<pos=".len();
        let digits = line[digits_start..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        let digits_end = digits_start + digits;
        if digits > 0 && line[digits_end..].starts_with("%>") {
            let percent = js_parse_int(&line[digits_start..digits_end]).unwrap_or(0.0);
            return Some((start, digits_end + 2, percent));
        }
        from = start + 1;
    }
    None
}

/// V1 `preprocessBootLog(rawLog)`: in each line, the first `<pos=N%>` becomes
/// `max(1, floor((1920 * N/100 - prefix.length * charWidth) / charWidth))` spaces; the line is
/// `split` on the tag text and only the first two parts are kept (V1 drops anything after a
/// repeated tag). A non-finite or non-positive `char_width` is replaced by 10 (V1 never passes
/// one, see [`normalize_boot_char_width`]).
pub fn preprocess_boot_log(raw_log: &str, char_width: f64) -> String {
    let width = if char_width.is_finite() && char_width > 0.0 {
        char_width
    } else {
        BOOT_CHAR_WIDTH_FALLBACK
    };
    raw_log
        .split('\n')
        .map(|line| {
            let Some((start, end, percent)) = find_pos_tag(line) else {
                return line.to_owned();
            };
            let tag = &line[start..end];
            let mut parts = line.split(tag);
            let prefix = parts.next().unwrap_or("");
            let suffix = parts.next().unwrap_or("");
            let target_x = BOOT_STAGE_WIDTH * (percent / 100.0);
            let prefix_width = utf16_len(prefix) as f64 * width;
            let spaces = ((target_x - prefix_width) / width)
                .floor()
                .max(1.0)
                .min(MAX_BOOT_PADDING as f64);
            format!("{prefix}{}{suffix}", " ".repeat(spaces as usize))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Delay after revealing a boot line (`line.includes("...") || line.trim() === ""` → 400 ms,
/// else 50 ms).
pub fn boot_line_delay_ms(line: &str) -> u32 {
    if line.contains("...") || js_trim(line).is_empty() {
        BOOT_SLOW_LINE_DELAY_MS
    } else {
        BOOT_FAST_LINE_DELAY_MS
    }
}

/// Memory counter at `elapsed_ms` of the animation: `floor(min(1, elapsed/1500) * 32767)`.
pub fn boot_memory_value(elapsed_ms: u32) -> u32 {
    let progress = (f64::from(elapsed_ms) / f64::from(BOOT_MEMORY_ANIMATION_MS)).min(1.0);
    (progress * f64::from(BOOT_LOG_MEM_MAX)).floor() as u32
}

/// One step of the desktop boot sequence: apply `text` (if `Some`) and `visible_chars`, then
/// wait `wait_after_ms` before the next step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootStep {
    /// New full (preprocessed) boot text, when it changes.
    pub text: Option<String>,
    /// Number of leading characters to show (V1 `displayText.slice(0, visibleChars)`; the text
    /// is ASCII, so this is also a byte count).
    pub visible_chars: usize,
    /// Pause after this step.
    pub wait_after_ms: u32,
}

/// The V1 `LoadingPanel` sequence (when `Config_SkipLoading != 1`) as data. The caller waits
/// [`BOOT_INITIAL_DELAY_MS`], plays the steps, then waits [`BOOT_LOGO_DELAY_MS`], shows the
/// logo, waits [`BOOT_FINISH_DELAY_MS`] and finishes. The first step sets the text with
/// nothing visible; the `Memory Initialize` line animates the counter 0 → 32767 in 51 frames.
pub fn boot_sequence(char_width: f64) -> Vec<BootStep> {
    let build = |memory: u32| preprocess_boot_log(&boot_log_with_memory(memory), char_width);
    let mut text = build(0);
    let mut steps = vec![BootStep {
        text: Some(text.clone()),
        visible_chars: 0,
        wait_after_ms: 0,
    }];
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let mut visible = 0usize;
    let mut line_index = 0;
    while line_index < lines.len() {
        let line = lines[line_index].clone();
        if line.contains("Memory Initialize") {
            let mut elapsed = 0;
            while elapsed <= BOOT_MEMORY_ANIMATION_MS {
                text = build(boot_memory_value(elapsed));
                lines = text.split('\n').map(str::to_owned).collect();
                visible = lines
                    .iter()
                    .take(line_index + 1)
                    .map(|l| utf16_len(l) + 1)
                    .sum();
                let wait_after_ms = if elapsed < BOOT_MEMORY_ANIMATION_MS {
                    BOOT_MEMORY_STEP_MS
                } else {
                    0
                };
                steps.push(BootStep {
                    text: Some(text.clone()),
                    visible_chars: visible,
                    wait_after_ms,
                });
                elapsed += BOOT_MEMORY_STEP_MS;
            }
        } else {
            visible += utf16_len(&line) + 1;
            steps.push(BootStep {
                text: None,
                visible_chars: visible,
                wait_after_ms: boot_line_delay_ms(&line),
            });
        }
        line_index += 1;
    }
    steps
}

/// `text.slice(0, visibleChars)` clamped to the text (and to a char boundary).
pub fn boot_visible_text(text: &str, visible_chars: usize) -> &str {
    let mut end = visible_chars.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// One change-log entry. V1 renders `t(key, "", lang).split("\n")` as `<li>` items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChangeLogEntry {
    /// Version label (`"Version 1.4"`).
    pub version: &'static str,
    /// Release date (`"YYYY. MM. DD"`).
    pub date: &'static str,
    /// Locale key of the bullet text (`changelog_v14`, …).
    pub key: &'static str,
    /// Built-in Japanese text (V2 entries only; V1 texts live in the locale files).
    pub builtin_ja: &'static str,
    /// Built-in English text (V2 entries only).
    pub builtin_en: &'static str,
}

impl ChangeLogEntry {
    /// Default text to pass as `t(key, default, lang)`'s default: `""` for V1 entries (exactly
    /// V1, whose six keys exist in all 13 locales), the built-in text for V2 entries
    /// (Japanese for language 0, English otherwise) so they render even without locale keys.
    pub fn default_text(&self, lang: i64) -> &'static str {
        if lang == 0 {
            self.builtin_ja
        } else {
            self.builtin_en
        }
    }

    /// True for the six entries carried over unchanged from V1.
    pub fn is_v1(&self) -> bool {
        self.builtin_ja.is_empty() && self.builtin_en.is_empty()
    }
}

/// Splits a change-log text into its `<li>` lines (`text.split("\n")`, empty lines kept).
pub fn change_log_lines(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

const fn v1_entry(version: &'static str, date: &'static str, key: &'static str) -> ChangeLogEntry {
    ChangeLogEntry {
        version,
        date,
        key,
        builtin_ja: "",
        builtin_en: "",
    }
}

/// Change log, newest first: the V2 (Rust) entry, then V1 `CHANGE_LOG_DATA` verbatim.
pub const CHANGE_LOG_DATA: [ChangeLogEntry; 7] = [
    ChangeLogEntry {
        version: "Version 2.0",
        date: "2026. 11. 23",
        key: "changelog_v20",
        builtin_ja: concat!(
            "アプリ全体をRustで全面的に書き直しました。\n",
            "Live2DレンダラーをRustで再実装しました。\n",
            "Live2Dの画質を改善しました。\n",
            "Web版では設定・記憶・アチーブメントがV1からそのまま引き継がれます。\n",
            "Web版のAPIはV1と同じ仕様を維持しています。\n",
            "設定にログイン画面スキップ機能を追加しました。\n",
            "初回起動時に国旗と言語のボタンから表示言語を選択できる画面を追加しました。\n",
            "CharacterMemoryベースの文脈記憶・RAG機能と、紅莉栖の詳細な人間関係・人格プロファイルを組み込みました。\n",
            "Gemini / FreeAPIの応答が文の途中で途切れる問題を修正しました。\n",
            "UIのバグを修正しました。"
        ),
        builtin_en: concat!(
            "Rewrote the entire application in Rust.\n",
            "Re-implemented the Live2D renderer in Rust.\n",
            "Improved Live2D image quality.\n",
            "On the web, settings, memory and achievements carry over from V1 unchanged.\n",
            "The web API keeps exactly the same contract as V1.\n",
            "Added a Skip Login Screen option to settings.\n",
            "Added a first-launch screen to select the display language using flag and language buttons.\n",
            "Integrated CharacterMemory-based contextual memory/RAG and detailed Kurisu persona and relationship profiles.\n",
            "Fixed Gemini / FreeAPI replies cutting off mid-sentence.\n",
            "Fixed UI bugs."
        ),
    },
    v1_entry("Version 1.4", "2026. 08. 01", "changelog_v14"),
    v1_entry("Version 1.3", "2026. 06. 16", "changelog_v13"),
    v1_entry("Version 1.2", "2026. 05. 04", "changelog_v12"),
    v1_entry("Version 1.1", "2026. 03. 21", "changelog_v11"),
    v1_entry("Version 1.0.1", "2026. 02. 23", "changelog_v101"),
    v1_entry("Version 1.0", "2026. 02. 22", "changelog_v10"),
];

/// The six V1 entries (V1 `CHANGE_LOG_DATA`, unchanged).
pub fn v1_change_log() -> &'static [ChangeLogEntry] {
    &CHANGE_LOG_DATA[1..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn sha256(s: &str) -> String {
        Sha256::digest(s.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Golden SHA-256 values computed from V1 with Node 22.
    #[test]
    fn boot_log_is_verbatim() {
        assert_eq!(BOOT_LOG.len(), 814);
        assert_eq!(BOOT_LOG.split('\n').count(), 41);
        assert_eq!(
            sha256(BOOT_LOG),
            "746ddd81f757635606e427e5c16e54025a3b681a173ea5549e03d646cdece7e0"
        );
        assert_eq!(
            sha256(&boot_log_with_memory(0)),
            "7857c859b7e4778cb3db002e9c27d91dd87f6e6cbd1eeb613c5f261cc02ca0a6"
        );
        assert_eq!(
            sha256(&boot_log_with_memory(32767)),
            "7210195c7270565b1ebeceb2d70613c21602837f7c1e22121acc5f90b9cbc05a"
        );
        assert_eq!(BOOT_LOG.matches("[MEM]").count(), 1);
        assert_eq!(BOOT_LOG.matches("<pos=30%>").count(), 11);
    }

    /// Golden: V1 `preprocessBootLog` output (Node 22).
    #[test]
    fn preprocess_golden() {
        let at_10_8 = preprocess_boot_log(&boot_log_with_memory(0), 10.8);
        let lines: Vec<&str> = at_10_8.split('\n').collect();
        assert_eq!(
            lines[25],
            format!("Mounting proc at /proc...{}[OK]", " ".repeat(28))
        );
        assert_eq!(
            lines[27],
            format!("Initakising network{}[OK]", " ".repeat(34))
        );
        assert_eq!(
            lines[32],
            format!("Starting system log at /log/sys...{}[OK]", " ".repeat(19))
        );
        assert_eq!(
            lines[33],
            format!("Cleaning /var/lock{}[OK]", " ".repeat(35))
        );
        let at_100 = preprocess_boot_log(&boot_log_with_memory(0), 100.0);
        assert!(
            at_100.contains("\nMounting sysfs at /sts... [OK]\n"),
            "at least one space"
        );
        let at_7 = preprocess_boot_log(&boot_log_with_memory(0), 7.0);
        assert!(at_7.contains(&format!("\nCleaning /tmp{}[OK]\n", " ".repeat(69))));
        let extra: [(&str, String); 5] = [
            ("a<pos=50%>b<pos=50%>c", format!("a{}b", " ".repeat(95))),
            ("<pos=10%>", " ".repeat(19)),
            ("x<pos=007%>y", format!("x{}y", " ".repeat(12))),
            ("no tag", "no tag".to_owned()),
            ("<pos=%>z", "<pos=%>z".to_owned()),
        ];
        for (line, expected) in extra {
            assert_eq!(preprocess_boot_log(line, 10.0), expected, "{line:?}");
        }
        // First failing `<pos=` is skipped; the tag found is `<pos=5%>` (96 px - 5 chars * 10).
        assert_eq!(
            preprocess_boot_log("<pos=<pos=5%>z", 10.0),
            format!("<pos={}z", " ".repeat(4))
        );
        assert_eq!(normalize_boot_char_width(None), 10.0);
        assert_eq!(normalize_boot_char_width(Some(0.0)), 10.0);
        assert_eq!(normalize_boot_char_width(Some(10.8)), 10.8);
        assert_eq!(
            preprocess_boot_log("a<pos=10%>b", 0.0),
            preprocess_boot_log("a<pos=10%>b", 10.0)
        );
    }

    /// Golden: the 51 memory values V1 shows (elapsed = 0, 30, …, 1500).
    #[test]
    fn memory_animation_golden() {
        let frames: Vec<u32> = (0..=50).map(|i| boot_memory_value(i * 30)).collect();
        let golden = [
            0, 655, 1310, 1966, 2621, 3276, 3932, 4587, 5242, 5898, 6553, 7208, 7864, 8519, 9174,
            9830, 10485, 11140, 11796, 12451, 13106, 13762, 14417, 15072, 15728, 16383, 17038,
            17694, 18349, 19004, 19660, 20315, 20970, 21626, 22281, 22936, 23592, 24247, 24902,
            25558, 26213, 26868, 27524, 28179, 28834, 29490, 30145, 30800, 31456, 32111, 32767,
        ];
        assert_eq!(frames, golden);
        assert_eq!(boot_memory_value(9999), 32767);
    }

    #[test]
    fn boot_sequence_shape() {
        let steps = boot_sequence(10.0);
        assert_eq!(steps.len(), 1 + 10 + 51 + 30);
        assert_eq!(steps[0].visible_chars, 0);
        assert_eq!(
            steps[1].visible_chars,
            "Amadeus System Ver 1.09.2 rev.2123".len() + 1
        );
        assert_eq!(steps[1].wait_after_ms, 50);
        assert_eq!(steps[2].wait_after_ms, 400, "contains ...");
        let first_mem = &steps[11];
        assert!(first_mem
            .text
            .as_deref()
            .is_some_and(|t| t.contains(">>Memory Initialize 0/32767MBytes")));
        assert_eq!(first_mem.wait_after_ms, 30);
        let last_mem = &steps[61];
        assert!(last_mem
            .text
            .as_deref()
            .is_some_and(|t| t.contains(">>Memory Initialize 32767/32767MBytes")));
        assert_eq!(last_mem.wait_after_ms, 0);
        let last = steps.last().map(|s| s.visible_chars);
        let final_text = last_mem.text.clone().unwrap_or_default();
        assert_eq!(
            last,
            Some(final_text.len() + 1),
            "every line revealed (+1 per newline, V1 counts one past the end)"
        );
        assert_eq!(boot_visible_text(&final_text, usize::MAX), final_text);
        assert_eq!(boot_visible_text("abc", 2), "ab");
        assert_eq!(boot_line_delay_ms("   "), 400);
        assert_eq!(boot_line_delay_ms("ROSS:"), 50);
    }

    #[test]
    fn change_log_keeps_v1_entries() {
        let v1: Vec<(&str, &str, &str)> = v1_change_log()
            .iter()
            .map(|e| (e.version, e.date, e.key))
            .collect();
        assert_eq!(
            v1,
            [
                ("Version 1.4", "2026. 08. 01", "changelog_v14"),
                ("Version 1.3", "2026. 06. 16", "changelog_v13"),
                ("Version 1.2", "2026. 05. 04", "changelog_v12"),
                ("Version 1.1", "2026. 03. 21", "changelog_v11"),
                ("Version 1.0.1", "2026. 02. 23", "changelog_v101"),
                ("Version 1.0", "2026. 02. 22", "changelog_v10"),
            ]
        );
        assert!(v1_change_log()
            .iter()
            .all(|e| e.is_v1() && e.default_text(0).is_empty() && e.default_text(1).is_empty()));
        let v2 = CHANGE_LOG_DATA[0];
        assert!(!v2.is_v1());
        assert!(v2.default_text(0).starts_with("アプリ全体をRust"));
        assert!(v2
            .default_text(5)
            .starts_with("Rewrote the entire application in Rust"));
        assert_eq!(change_log_lines(v2.builtin_ja).len(), 10);
        assert_eq!(change_log_lines(v2.builtin_en).len(), 10);
        assert_eq!(change_log_lines("a\n\nb"), ["a", "", "b"]);
    }

    /// The V2 entry carries the 2.0.0 release date (CHANGELOG.md `## [2.0.0] - 2026-11-23`);
    /// every date keeps V1's `YYYY. MM. DD` spelling and the list stays newest first.
    #[test]
    fn change_log_dates() {
        let v2 = CHANGE_LOG_DATA[0];
        assert_eq!(
            (v2.version, v2.date, v2.key),
            ("Version 2.0", "2026. 11. 23", "changelog_v20")
        );
        for entry in &CHANGE_LOG_DATA {
            let b = entry.date.as_bytes();
            let digits = |range: std::ops::Range<usize>| b[range].iter().all(u8::is_ascii_digit);
            assert!(
                b.len() == 12
                    && digits(0..4)
                    && &b[4..6] == b". "
                    && digits(6..8)
                    && &b[8..10] == b". "
                    && digits(10..12),
                "{:?}",
                entry.date
            );
        }
        assert!(
            CHANGE_LOG_DATA.windows(2).all(|w| w[0].date > w[1].date),
            "newest first"
        );
    }
}
