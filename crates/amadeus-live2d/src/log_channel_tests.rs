//! The `cubism` log target is the framework's `CubismLog*` channel. V1 started the framework with
//! `option.loggingLevel = LogLevel_Off`, so those lines never reached the console; the app's
//! logger (`apps/web/src/logger.rs`) gives this target its own level, Off by default (checklist
//! L2D-015). These tests pin which messages use the channel: V1 `CubismLog*` calls do, and so do
//! V2-only render notes for cases where V1 printed nothing (a clipping-mask render target that
//! cannot be created, an unbound model texture). The framework's V1 `console.warn` sites and V2's
//! documented diagnostics (DECISIONS D4, V1_DIFFERENCES 9.1 and 9.12) do not.
//!
//! A source scan lists every `log` macro call in the crate's non-test code, in any spelling
//! (with or without a path, any spacing and delimiter), and fails on imports of `log` macros, so
//! a new line that reaches the console fails a test unless it is listed in `VISIBLE_LOG_LINES`.

use std::cell::RefCell;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Once;

use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::framework::motion::CubismExpressionMotionManager;
use crate::framework::CubismUserModel;
use crate::math::cubism_math;
use crate::render::clipping::tests::MockModel;
use crate::render::clipping::ClippingManager;

/// Records `(level, target, message)` for the threads that asked for it.
struct Capture;

thread_local! {
    static CAPTURED: RefCell<Option<Vec<(Level, String, String)>>> = const { RefCell::new(None) };
}

impl Log for Capture {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        CAPTURED.with(|captured| {
            if let Some(records) = captured.borrow_mut().as_mut() {
                records.push((
                    record.level(),
                    record.target().to_owned(),
                    record.args().to_string(),
                ));
            }
        });
    }

    fn flush(&self) {}
}

/// The records `f` logs on this thread (tests run in parallel; capture is per thread).
pub(crate) fn captured(f: impl FnOnce()) -> Vec<(Level, String, String)> {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        if log::set_logger(&Capture).is_ok() {
            log::set_max_level(LevelFilter::Trace);
        }
    });
    CAPTURED.with(|c| *c.borrow_mut() = Some(Vec::new()));
    f();
    CAPTURED.with(|c| c.borrow_mut().take().unwrap_or_default())
}

pub(crate) fn record(level: Level, target: &str, message: &str) -> (Level, String, String) {
    (level, target.to_owned(), message.to_owned())
}

#[test]
fn physics_load_failure_is_on_the_cubism_channel() {
    // The L2D-015 case: physics3.json unavailable → V1 `CubismLogError('Failed to loadPhysics().')`.
    let records = captured(|| {
        let mut model = CubismUserModel::new();
        assert!(model.load_physics(&[]).is_err());
    });
    assert_eq!(
        records,
        vec![record(Level::Error, "cubism", "Failed to loadPhysics().")]
    );
}

#[test]
fn render_layer_cubism_log_calls_are_on_the_cubism_channel() {
    // V1 `CubismLogWarning` in `CubismClippingManager.initializeForDrawable`.
    let records = captured(|| {
        ClippingManager::new().initialize_for_drawable(&MockModel::with_drawables(2), 0);
    });
    assert_eq!(
        records,
        vec![record(
            Level::Warn,
            "cubism",
            "The number of render textures must be an integer greater than or equal to 1. Set the number of render textures to 1."
        )]
    );
}

#[test]
fn v1_console_warn_sites_stay_off_the_cubism_channel() {
    // V1 printed these with `console.warn` (not `CubismLog*`), so they must stay visible.
    let records = captured(|| {
        assert!(cubism_math::modulo(f64::INFINITY, 2.0).is_nan());
        let mut manager = CubismExpressionMotionManager::new();
        assert_eq!(manager.fade_weight(3), -1.0);
        manager.set_fade_weight(3, 0.5);
    });
    let targets: Vec<(Level, &str)> = records
        .iter()
        .map(|(level, target, _)| (*level, target.as_str()))
        .collect();
    assert_eq!(
        targets,
        vec![
            (Level::Warn, "amadeus_live2d::math::cubism_math"),
            (
                Level::Warn,
                "amadeus_live2d::framework::motion::expression_manager"
            ),
            (
                Level::Warn,
                "amadeus_live2d::framework::motion::expression_manager"
            ),
        ]
    );
    assert_eq!(
        records[0].2,
        "divided: Infinity, divisor: 2 mod() returns 'NaN'."
    );
    assert_eq!(
        records[1].2,
        "Failed to get the fade weight value. The element at that index does not exist."
    );
    assert_eq!(
        records[2].2,
        "Failed to set the fade weight value. The element at that index does not exist."
    );
}

// ---- Source scan: every `log` macro call in the crate's non-test sources ----
//
// Plain std code down to `scan_crate`, so the scan can also be compiled on its own and run on
// other source trees.

/// The macros of the `log` crate. A call counts in every spelling the compiler accepts: any
/// path (`log::warn!`, `::log::warn!`, `warn!` after an import), white space around the `!` and
/// any delimiter (`(…)`, `[…]`, `{…}`). Text in comments and literals does not count.
const LOG_MACROS: [&str; 6] = ["error", "warn", "info", "debug", "trace", "log"];

/// The only `log` calls of this crate that may reach the console (no `cubism` target), as
/// `(start of the format string, number of calls, why it is visible)`: 8 calls in all. Every
/// other record is a line V1 never printed and belongs on `target: "cubism"`, unless a
/// documented V1 difference says otherwise.
const VISIBLE_LOG_LINES: &[(&str, usize, &str)] = &[
    (
        "divided: {}, divisor: {} mod() returns 'NaN'.",
        1,
        "V1 console.warn in CubismMath.mod",
    ),
    (
        "Failed to get the fade weight value.",
        1,
        "V1 console.warn in CubismExpressionMotionManager.getFadeWeight",
    ),
    (
        "Failed to set the fade weight value.",
        2,
        "V1 console.warn in CubismExpressionMotionManager.setFadeWeight/updateMotion",
    ),
    (
        "Cubism program {which:?} is unusable",
        1,
        "D4, V1_DIFFERENCES 9.1: a shader compile failure is logged once",
    ),
    (
        "Live2D mask shaders are unavailable",
        1,
        "D4, V1_DIFFERENCES 9.1: masked parts are drawn without clipping",
    ),
    (
        "Live2D drawable shader is unavailable",
        1,
        "D4, V1_DIFFERENCES 9.1: logged once",
    ),
    (
        "This model uses Cubism 5.3 blend modes/offscreens",
        1,
        "V1_DIFFERENCES 9.12: one warning at load",
    ),
];

/// One `log` macro call found in the sources.
#[derive(Debug)]
struct LogCall {
    /// Path relative to `src/`.
    file: String,
    /// The macro: `"error"` … `"trace"`, or `"log"`.
    level: &'static str,
    /// The call has `target: "cubism"` (or a `cubism::…` sub-target).
    cubism: bool,
    /// The first string literal after the target (the format string), as written.
    message: String,
}

/// What [`scan_dir`] found in a crate's non-test sources.
#[derive(Debug, Default)]
struct Scan {
    /// The files scanned, relative to `src/`; the files of `#[cfg(test)]` modules are left out.
    files: Vec<String>,
    calls: Vec<LogCall>,
    /// `use` declarations (and `extern crate log`) that put `log` macros in scope, possibly
    /// under another name the call scan cannot see, as `file: declaration`.
    macro_imports: Vec<String>,
}

/// A source file split into code, comments and literals.
struct Lexed {
    /// The file as written.
    text: String,
    /// `text` with every comment and the contents of every string, raw-string and char literal
    /// replaced by spaces (line breaks kept), so code patterns never match inside them. Byte
    /// offsets are those of `text`.
    code: String,
    /// The byte ranges of the string literals' contents (between the quotes), in order.
    strings: Vec<Range<usize>>,
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn skip_ws(code: &str, mut i: usize) -> usize {
    while code.as_bytes().get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

/// Replaces the bytes of `range` (which splits no character) with spaces, except line breaks.
fn blank(code: &mut [u8], range: Range<usize>) {
    for b in &mut code[range] {
        if *b != b'\n' {
            *b = b' ';
        }
    }
}

/// `r"`, `r#"`, `br##"`, `cr"`, … at the start of `rest`: the length of the opening delimiter
/// and the number of `#`.
fn raw_string_start(rest: &[u8]) -> Option<(usize, usize)> {
    let prefix = usize::from(matches!(rest.first(), Some(b'b' | b'c')));
    if rest.get(prefix) != Some(&b'r') {
        return None;
    }
    let hashes = rest[prefix + 1..]
        .iter()
        .take_while(|&&b| b == b'#')
        .count();
    (rest.get(prefix + 1 + hashes) == Some(&b'"')).then_some((prefix + hashes + 2, hashes))
}

/// The length of the char literal at the start of `s` (which starts with `'`), or `None` for a
/// lifetime or a label.
fn char_literal_len(s: &str) -> Option<usize> {
    let mut chars = s.char_indices().skip(1);
    let (_, first) = chars.next()?;
    if first == '\\' {
        // `'\n'`, `'\''`, `'\x7f'`, `'\u{1F600}'`
        chars.next()?;
        return chars.find(|&(_, c)| c == '\'').map(|(at, _)| at + 1);
    }
    let (at, second) = chars.next()?;
    (second == '\'').then_some(at + 1)
}

/// Splits `text` into code, comments and literals: enough of Rust's lexical grammar for this
/// crate (nested block comments, escapes, raw strings, `b`/`c` prefixes, char literals versus
/// lifetimes).
fn lex(text: &str) -> Lexed {
    let bytes = text.as_bytes();
    let mut code = bytes.to_vec();
    let mut strings = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        let after_ident = i > 0 && is_ident_byte(bytes[i - 1]);
        if rest.starts_with(b"//") {
            let end = rest
                .iter()
                .position(|&b| b == b'\n')
                .map_or(bytes.len(), |n| i + n);
            blank(&mut code, i..end);
            i = end;
        } else if rest.starts_with(b"/*") {
            let mut depth = 0usize;
            let mut j = i;
            loop {
                if bytes[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if bytes[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    assert!(j < bytes.len(), "unterminated block comment at byte {i}");
                    j += 1;
                }
            }
            blank(&mut code, i..j);
            i = j;
        } else if let Some((open, hashes)) = raw_string_start(rest).filter(|_| !after_ident) {
            let start = i + open;
            let terminator = format!("\"{}", "#".repeat(hashes));
            let end = start
                + text[start..]
                    .find(&terminator)
                    .unwrap_or_else(|| panic!("unterminated raw string at byte {i}"));
            strings.push(start..end);
            blank(&mut code, start..end);
            i = end + terminator.len();
        } else if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while bytes.get(j).is_some_and(|&b| b != b'"') {
                j += if bytes[j] == b'\\' { 2 } else { 1 };
            }
            assert!(j < bytes.len(), "unterminated string at byte {i}");
            strings.push(start..j);
            blank(&mut code, start..j);
            i = j + 1;
        } else if bytes[i] == b'\'' {
            match char_literal_len(&text[i..]) {
                Some(len) => {
                    blank(&mut code, i + 1..i + len - 1);
                    i += len;
                }
                // A lifetime or a label.
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    Lexed {
        text: text.to_owned(),
        code: String::from_utf8(code).expect("blanking keeps UTF-8"),
        strings,
    }
}

/// The index of the bracket that closes the one at `open`.
fn closing(code: &str, open: usize) -> usize {
    let mut depth = 0usize;
    for (i, b) in code.bytes().enumerate().skip(open) {
        match b {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced brackets from byte {open}");
}

/// The identifiers and keywords of `code`, with their byte offsets.
fn identifiers(code: &str) -> impl Iterator<Item = (usize, &str)> + '_ {
    let bytes = code.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < bytes.len() {
            let start = i;
            while i < bytes.len() && is_ident_byte(bytes[i]) {
                i += 1;
            }
            if i > start && !bytes[start].is_ascii_digit() {
                return Some((start, &code[start..i]));
            }
            i = i.max(start + 1);
        }
        None
    })
}

/// The inline `#[cfg(test)] mod … { … }` items of `code` (byte ranges) and the names of its
/// `#[cfg(test)] mod name;` declarations (test modules in files of their own).
fn test_modules(code: &str) -> (Vec<Range<usize>>, Vec<String>) {
    const CFG_TEST: &str = "#[cfg(test)]";
    let bytes = code.as_bytes();
    let mut inline = Vec::new();
    let mut files = Vec::new();
    for (at, _) in code.match_indices(CFG_TEST) {
        // Further attributes and a visibility may come before `mod`.
        let mut i = skip_ws(code, at + CFG_TEST.len());
        loop {
            if code[i..].starts_with("#[") {
                i = skip_ws(code, closing(code, i + 1) + 1);
            } else if code[i..].starts_with("pub")
                && !bytes.get(i + 3).copied().is_some_and(is_ident_byte)
            {
                i = skip_ws(code, i + 3);
                if bytes.get(i) == Some(&b'(') {
                    i = skip_ws(code, closing(code, i) + 1);
                }
            } else {
                break;
            }
        }
        if !code[i..].starts_with("mod") || !bytes.get(i + 3).is_some_and(u8::is_ascii_whitespace) {
            continue;
        }
        let name_start = skip_ws(code, i + 3);
        let name_end = (name_start..bytes.len())
            .find(|&j| !is_ident_byte(bytes[j]))
            .unwrap_or(bytes.len());
        let after = skip_ws(code, name_end);
        match bytes.get(after) {
            Some(b';') => files.push(code[name_start..name_end].to_owned()),
            Some(b'{') => inline.push(at..closing(code, after) + 1),
            _ => {}
        }
    }
    (inline, files)
}

/// The code of `lexed` without its inline `#[cfg(test)]` modules.
fn non_test_code(lexed: &Lexed) -> String {
    let mut code = lexed.code.clone().into_bytes();
    for range in test_modules(&lexed.code).0 {
        blank(&mut code, range);
    }
    String::from_utf8(code).expect("blanking keeps UTF-8")
}

/// The `log` macro calls in `code` (the code of `lexed`, possibly with more blanked out).
fn calls_in(file: &str, lexed: &Lexed, code: &str) -> Vec<LogCall> {
    let bytes = code.as_bytes();
    let mut calls = Vec::new();
    for (start, word) in identifiers(code) {
        let Some(level) = LOG_MACROS.into_iter().find(|name| *name == word) else {
            continue;
        };
        let bang = skip_ws(code, start + word.len());
        let open = skip_ws(code, bang + 1);
        if bytes.get(bang) != Some(&b'!') || !matches!(bytes.get(open), Some(b'(' | b'[' | b'{')) {
            continue;
        }
        let close = closing(code, open);
        let literals: Vec<&str> = lexed
            .strings
            .iter()
            .filter(|range| open < range.start && range.end < close)
            .map(|range| &lexed.text[range.clone()])
            .collect();
        // `target: "…"`; a target that is not a literal counts as visible.
        let literal_target = code[open + 1..close]
            .trim_start()
            .strip_prefix("target")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix(':'))
            .filter(|rest| !rest.starts_with(':'))
            .map(str::trim_start)
            .is_some_and(|rest| {
                rest.starts_with('"') || raw_string_start(rest.as_bytes()).is_some()
            });
        let cubism = literal_target
            && literals
                .first()
                .is_some_and(|target| *target == "cubism" || target.starts_with("cubism::"));
        let message = literals
            .get(usize::from(literal_target))
            .map_or_else(String::new, |message| (*message).to_owned());
        calls.push(LogCall {
            file: file.to_owned(),
            level,
            cubism,
            message,
        });
    }
    calls
}

/// The leaf paths of a use tree: `log::{self, warn as w}` gives `[log, self]` and `[log, warn]`;
/// a glob ends in `*`, and raw identifiers lose their `r#` (`log::r#warn` gives `[log, warn]`).
fn use_tree_leaves(tree: &str) -> Vec<Vec<String>> {
    let mut tokens = Vec::new();
    let mut rest = tree.trim_start();
    while let Some(first) = rest.chars().next() {
        let len = if rest.starts_with("::") {
            2
        } else if "{},*".contains(first) {
            1
        } else {
            rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '#'))
                .unwrap_or(rest.len())
                .max(first.len_utf8())
        };
        tokens.push(&rest[..len]);
        rest = rest[len..].trim_start();
    }
    let mut leaves = Vec::new();
    use_subtree(&tokens, &mut 0, Vec::new(), &mut leaves);
    leaves
}

fn use_subtree(
    tokens: &[&str],
    pos: &mut usize,
    mut path: Vec<String>,
    leaves: &mut Vec<Vec<String>>,
) {
    while let Some(&token) = tokens.get(*pos) {
        *pos += 1;
        match token {
            "::" => {}
            "*" => {
                path.push(token.to_owned());
                break;
            }
            "{" => {
                while tokens.get(*pos).is_some_and(|&t| t != "}") {
                    use_subtree(tokens, pos, path.clone(), leaves);
                    if tokens.get(*pos) == Some(&",") {
                        *pos += 1;
                    }
                }
                *pos += 1;
                return;
            }
            name => {
                path.push(name.strip_prefix("r#").unwrap_or(name).to_owned());
                match tokens.get(*pos) {
                    Some(&"::") => {}
                    Some(&"as") => {
                        *pos += 2;
                        break;
                    }
                    _ => break,
                }
            }
        }
    }
    leaves.push(path);
}

/// The `use` declarations of `code` that import a `log` macro (or glob-import from `log`), and
/// any `extern crate log`.
fn macro_imports_in(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (start, word) in identifiers(code) {
        if word != "use" && word != "extern" {
            continue;
        }
        // Neither declaration contains a `;`.
        let end = code[start..].find(';').map_or(code.len(), |n| start + n);
        let decl = code[start..end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let imports_macro = if word == "use" {
            use_tree_leaves(&code[start + word.len()..end])
                .iter()
                .any(|path| {
                    path.len() >= 2
                        && path[0] == "log"
                        && path
                            .last()
                            .is_some_and(|last| last == "*" || LOG_MACROS.contains(&last.as_str()))
                })
        } else {
            decl == "extern crate log" || decl.starts_with("extern crate log as ")
        };
        if imports_macro {
            found.push(format!("{decl};"));
        }
    }
    found
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

/// The module path of a file relative to `src/`: `app/mod.rs` → `app`, `app/driver.rs` →
/// `app/driver`, `lib.rs` → the empty root.
fn module_path(file: &str) -> &str {
    let path = file.strip_suffix(".rs").unwrap_or(file);
    match path {
        "lib" | "main" => "",
        _ => path.strip_suffix("/mod").unwrap_or(path),
    }
}

/// Scans the `.rs` files under `src` (a crate's `src/`), leaving out test code: the files of
/// modules declared `#[cfg(test)] mod name;` (and their submodules) and inline
/// `#[cfg(test)] mod … { … }` items.
fn scan_dir(src: &Path) -> Scan {
    let mut paths = Vec::new();
    rust_files(src, &mut paths);
    paths.sort();
    let files: Vec<(String, Lexed)> = paths
        .iter()
        .map(|path| {
            let file = path
                .strip_prefix(src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (
                file,
                lex(&fs::read_to_string(path).expect("readable source file")),
            )
        })
        .collect();
    let mut test_only = Vec::new();
    for (file, lexed) in &files {
        let parent = module_path(file);
        for name in test_modules(&lexed.code).1 {
            test_only.push(if parent.is_empty() {
                name
            } else {
                format!("{parent}/{name}")
            });
        }
    }
    let mut scan = Scan::default();
    for (file, lexed) in &files {
        let module = module_path(file);
        if test_only
            .iter()
            .any(|test| module == test || module.starts_with(&format!("{test}/")))
        {
            continue;
        }
        let code = non_test_code(lexed);
        scan.calls.extend(calls_in(file, lexed, &code));
        scan.macro_imports.extend(
            macro_imports_in(&code)
                .into_iter()
                .map(|decl| format!("{file}: {decl}")),
        );
        scan.files.push(file.clone());
    }
    scan
}

fn scan_crate() -> Scan {
    scan_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
}

#[test]
fn console_visible_log_calls_are_v1_console_lines_or_documented_diagnostics() {
    let scan = scan_crate();
    assert!(
        scan.files.iter().any(|f| f == "render/renderer.rs")
            && !scan.files.iter().any(|f| f.ends_with("_tests.rs")),
        "the scan covers the crate without its test modules: {:?}",
        scan.files
    );
    let cubism = scan.calls.iter().filter(|c| c.cubism).count();
    assert!(
        cubism >= 40,
        "the scan must see the framework's CubismLog* calls ({cubism} found)"
    );
    let visible: Vec<&LogCall> = scan.calls.iter().filter(|c| !c.cubism).collect();
    let unexpected: Vec<&&LogCall> = visible
        .iter()
        .filter(|c| {
            !VISIBLE_LOG_LINES
                .iter()
                .any(|(prefix, _, _)| c.message.starts_with(prefix))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "these log records reach the console although V1 printed nothing there: put them on \
         `target: \"cubism\"`, or document them as a V1 difference and list them in \
         VISIBLE_LOG_LINES: {unexpected:#?}"
    );
    for (prefix, count, why) in VISIBLE_LOG_LINES {
        let found = visible
            .iter()
            .filter(|c| c.message.starts_with(prefix))
            .count();
        assert_eq!(
            found, *count,
            "{prefix:?} ({why}): {found} visible calls, {count} listed; update VISIBLE_LOG_LINES"
        );
    }
    assert_eq!(visible.len(), 8, "{visible:#?}");
}

#[test]
fn log_macros_are_never_imported() {
    // An imported macro can be called under any name (`use log::warn as note;`), which the call
    // scan would not see.
    let scan = scan_crate();
    assert!(
        scan.macro_imports.is_empty(),
        "call the log macros by path (`log::warn!(target: \"cubism\", …)`) instead of importing \
         them: {:#?}",
        scan.macro_imports
    );
}

#[test]
fn v2_only_render_notes_are_on_the_cubism_channel() {
    // V1 printed nothing when a clipping-mask render target could not be created (it ignored
    // `createRenderTarget`'s result) or when a drawable's texture was unbound (null texture).
    let calls = scan_crate().calls;
    for (file, level, message) in [
        (
            "render/renderer.rs",
            "error",
            "Failed to create the clipping mask render target: {e}",
        ),
        (
            "render/renderer.rs",
            "warn",
            "Live2D texture {texture_no} is not bound; the drawables using it render black.",
        ),
        (
            "render/render_target.rs",
            "error",
            "Failed to create framebuffer",
        ),
        (
            "render/render_target.rs",
            "error",
            "Framebuffer is not complete",
        ),
    ] {
        let found: Vec<&LogCall> = calls
            .iter()
            .filter(|c| c.file == file && c.message == message)
            .collect();
        assert_eq!(found.len(), 1, "{file}: {message:?}: {found:#?}");
        assert!(
            found[0].cubism && found[0].level == level,
            "{file}: {message:?} must be log::{level}! on the cubism channel: {:#?}",
            found[0]
        );
    }
}

/// The calls and macro imports the scan finds in one file's source, test modules left out.
fn scan_source(text: &str) -> (Vec<(&'static str, bool, String)>, Vec<String>) {
    let lexed = lex(text);
    let code = non_test_code(&lexed);
    let calls = calls_in("x.rs", &lexed, &code)
        .into_iter()
        .map(|c| (c.level, c.cubism, c.message))
        .collect();
    (calls, macro_imports_in(&code))
}

#[test]
fn scan_sees_every_macro_spelling() {
    let source = r##"
        fn f() {
            log::warn!("a");
            ::log::error!("b");
            warn!("c");
            log::info! ("d");
            log::debug!{"e"}
            log::trace!["f"];
            log::log!(log::Level::Warn, "g {}", ')');
            log :: warn ! ( "h" );
            r#warn!("i");
            log::warn!(target: "cubism", "j");
            log::error!(target : "cubism::core", r#"k "quoted" (paren"#);
            log::warn!(target: "amadeus", "l");
            log::warn!(target: CUBISM, "m");
            let a = warn != b; // not a call
            my_warn!("n"); // another macro
            debug_assert!(true);
            // log::warn!("in a comment");
            /* log::warn!("in a /* nested */ block comment"); */
            let s = "log::warn!(\"in a string\") \\";
            let r = r#"log::warn!("in a raw string")"#;
            let c = '"';
            let lifetime: &'static str = "";
        }
        mod inner {
            fn g() { log::warn!("o"); }
        }
        #[cfg(test)]
        #[allow(dead_code)]
        pub(crate) mod tests {
            fn t() { log::warn!("in a test module"); }
        }
    "##;
    let expected = [
        ("warn", false, "a"),
        ("error", false, "b"),
        ("warn", false, "c"),
        ("info", false, "d"),
        ("debug", false, "e"),
        ("trace", false, "f"),
        ("log", false, "g {}"),
        ("warn", false, "h"),
        ("warn", false, "i"),
        ("warn", true, "j"),
        ("error", true, r#"k "quoted" (paren"#),
        ("warn", false, "l"),
        ("warn", false, "m"),
        ("warn", false, "o"),
    ];
    let (calls, imports) = scan_source(source);
    let calls: Vec<(&str, bool, &str)> = calls
        .iter()
        .map(|(level, cubism, message)| (*level, *cubism, message.as_str()))
        .collect();
    assert_eq!(calls, expected);
    assert!(imports.is_empty(), "{imports:?}");
}

#[test]
fn scan_flags_every_log_macro_import() {
    for source in [
        "use log::warn;",
        "use log::{self, warn};",
        "use log::{Level, warn as note};",
        "use log::*;",
        "use ::log::{\n    info,\n};",
        "pub(crate) use log::log;",
        "use {log::debug};",
        "use log::{Level, __private_api::*};",
        "use log::r#warn as note;",
        "use r#log::{r#error};",
        "#[macro_use]\nextern crate log;",
    ] {
        assert_eq!(scan_source(source).1.len(), 1, "{source}");
    }
    for source in [
        "use log::{Level, LevelFilter, Log, Metadata, Record};",
        "use log;",
        "use log as logging;",
        "use crate::render::warn;",
        "// use log::warn;",
        "#[cfg(test)]\nmod tests {\n    use log::warn;\n}",
        "extern \"C\" { fn warn(); }",
    ] {
        assert!(scan_source(source).1.is_empty(), "{source}");
    }
}
