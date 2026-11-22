//! `cargo xtask check-classes`: regression guard for Tailwind arbitrary-value classes
//! (PAGE-051).
//!
//! Tailwind generates a class only when it finds it as a complete literal in an `@source` file
//! (`apps/web/style/tailwind.css`) and accepts its value. A class assembled with `format!`, split
//! across literals or written with a value Tailwind rejects still compiles; it just renders
//! unstyled. This guard takes every class token that carries an arbitrary value, in its utility
//! (`text-[#d49e38]`, `border-[#d49e38]/20`, `hover:bg-[#d49e38]`, `md:h-[calc(100vh-5rem)]`,
//! `-left-[9px]`, `bg-(--brand)`, `text-sm/[17px]`) or in a variant
//! (`[&::-webkit-details-marker]:hidden`, `[@supports(display:grid)]:grid`,
//! `data-[state=open]:flex`, `max-[600px]:hidden`, `@min-[475px]:flex-row`), from the string
//! literals in [`SOURCE_DIRS`] and requires its selector, escaped like Tailwind 4 escapes class
//! names (CSSOM `CSS.escape`: a backslash before every character outside `[A-Za-z0-9_-]`), in one
//! of the dist's `*.css` files. Comments and test code are skipped: the code a `#[cfg(test)]`
//! attribute applies to (an item or statement, or just the field, enum variant, match arm,
//! argument or parameter it marks, never the code after it), `#![cfg(test)]` modules, the files
//! of test modules declared out of line (`#[cfg(test)] mod boot_tests;` → `boot_tests.rs`) with
//! everything below them, `tests.rs` files and `tests/` directories. When the end of the marked
//! code is ambiguous (a `>` comparison, an or-pattern's `|`), the skip stops early, so some test
//! code may be scanned but no later code is skipped. A test module with a `#[path]` attribute is
//! not followed: its file is scanned like any other. A fixture class in scanned test code is
//! reported like any other ([`NOT_CLASSES`]).
//!
//! `cargo xtask check` runs it right after its Trunk build; the CI web job runs it on the output
//! of `cargo xtask dist web`. Two shapes Tailwind accepts are deliberately not matched, because
//! ordinary text has them too: an arbitrary variant that is a bare selector (`[p]:flex`; only
//! `[&…]:` and `[@…]:` count, so bracketed text such as `[SMILE]:` never does) and arbitrary
//! properties (`[mask-type:luminance]`, shaped like log prefixes such as `[chat:stream]`). The
//! sources use neither.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::util;

/// Directories (relative to the workspace root) whose `.rs` files are scanned: the Rust
/// `@source` globs of `apps/web/style/tailwind.css` (the UI pages and app, the core's shared
/// class strings, the web entry point). Test code in them is skipped (see [`collect`]).
pub const SOURCE_DIRS: [&str; 3] = [
    "crates/amadeus-ui/src",
    "crates/amadeus-core/src",
    "apps/web/src",
];

/// Default dist directory: what `trunk build` writes in `apps/web` (Trunk.toml `dist`).
pub const DEFAULT_DIST: &str = "apps/web/dist";

/// Tokens that look like arbitrary-value classes but are not classes (e.g. prose that happens to
/// have the shape); they are never checked.
pub const NOT_CLASSES: &[&str] = &[];

/// One occurrence of a class in the sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Use {
    /// Path relative to the scanned root, with `/` separators.
    pub file: String,
    /// 1-based line.
    pub line: usize,
}

/// Every distinct class found, with its occurrences.
pub type Classes = BTreeMap<String, Vec<Use>>;

/// Result of [`check`].
#[derive(Debug)]
pub struct Outcome {
    /// Every class found in the sources.
    pub classes: Classes,
    /// Classes whose selector is in none of the CSS files.
    pub missing: Vec<String>,
    /// The CSS files that were searched.
    pub css_files: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------------------------
// Rust string literals
// ---------------------------------------------------------------------------------------------

/// The token kinds the extractor needs.
#[derive(Debug, PartialEq, Eq)]
enum Tok {
    Ident(String),
    Punct(u8),
    /// A string literal (escapes decoded) and the line it starts on.
    Str(usize, String),
    /// Numbers, char literals, lifetimes, non-ASCII identifiers.
    Other,
}

struct Lexer<'a> {
    src: &'a str,
    b: &'a [u8],
    i: usize,
    line: usize,
}

impl Lexer<'_> {
    fn peek(&self, k: usize) -> Option<u8> {
        self.b.get(self.i + k).copied()
    }

    /// Consumes one character (any width), counting lines.
    fn next_char(&mut self) -> Option<char> {
        let c = self.src.get(self.i..)?.chars().next()?;
        self.i += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn eat_ident_chars(&mut self) {
        while self
            .peek(0)
            .is_some_and(|c| c == b'_' || c.is_ascii_alphanumeric())
        {
            self.i += 1;
        }
    }

    /// `/* … */`, nested like Rust.
    fn block_comment(&mut self) {
        self.i += 2;
        let mut depth = 1usize;
        while depth > 0 {
            if self.b[self.i..].starts_with(b"/*") {
                depth += 1;
                self.i += 2;
            } else if self.b[self.i..].starts_with(b"*/") {
                depth -= 1;
                self.i += 2;
            } else if self.next_char().is_none() {
                break;
            }
        }
    }

    /// Body of a `"…"` literal after the opening quote, escapes decoded.
    fn cooked(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek(0) {
            match c {
                b'"' => {
                    self.i += 1;
                    break;
                }
                b'\\' => {
                    self.i += 1;
                    match self.peek(0) {
                        Some(b'n') => out.push('\n'),
                        Some(b't') => out.push('\t'),
                        Some(b'r') => out.push('\r'),
                        Some(b'0') => out.push('\0'),
                        Some(b'x') => {
                            let hex = self.src.get(self.i + 1..self.i + 3).unwrap_or_default();
                            if let Ok(v) = u8::from_str_radix(hex, 16) {
                                out.push(char::from(v));
                                self.i += 2;
                            }
                        }
                        Some(b'u') if self.peek(1) == Some(b'{') => {
                            let body = &self.src[self.i + 2..];
                            if let Some(close) = body.find('}') {
                                let digits: String =
                                    body[..close].chars().filter(|c| *c != '_').collect();
                                if let Some(ch) = u32::from_str_radix(&digits, 16)
                                    .ok()
                                    .and_then(char::from_u32)
                                {
                                    out.push(ch);
                                }
                                self.i += 2 + close;
                            }
                        }
                        // Line continuation: the newline and the next line's indentation vanish.
                        Some(b'\n' | b'\r') => {
                            while self.peek(0).is_some_and(|c| c.is_ascii_whitespace()) {
                                self.next_char();
                            }
                            continue;
                        }
                        Some(_) => {
                            // `\\`, `\"`, `\'` (and anything else) stand for the character itself.
                            if let Some(ch) = self.next_char() {
                                out.push(ch);
                            }
                            continue;
                        }
                        None => break,
                    }
                    self.i += 1;
                }
                _ => {
                    if let Some(ch) = self.next_char() {
                        out.push(ch);
                    }
                }
            }
        }
        out
    }

    /// Body of `r#"…"#` after the opening quote (`hashes` = number of `#`).
    fn raw(&mut self, hashes: usize) -> String {
        let start = self.i;
        loop {
            match self.peek(0) {
                None => return self.src[start..].to_owned(),
                Some(b'"')
                    if self
                        .b
                        .get(self.i + 1..self.i + 1 + hashes)
                        .is_some_and(|tail| tail.iter().all(|&c| c == b'#')) =>
                {
                    let text = self.src[start..self.i].to_owned();
                    self.i += 1 + hashes;
                    return text;
                }
                Some(_) => {
                    self.next_char();
                }
            }
        }
    }

    /// At a `'`: a char literal (`'x'`, `'"'`, `'\''`, `'\u{1F600}'`) or a lifetime / label.
    fn char_or_lifetime(&mut self) {
        self.i += 1;
        match self.peek(0) {
            Some(b'\\') => {
                self.i += 1;
                self.next_char();
                while let Some(c) = self.peek(0) {
                    if c == b'\n' {
                        break;
                    }
                    self.next_char();
                    if c == b'\'' {
                        break;
                    }
                }
            }
            Some(_) => {
                let start = self.i;
                let ch = self.next_char();
                if ch != Some('\n') && self.peek(0) == Some(b'\'') {
                    self.i += 1;
                } else {
                    self.i = start;
                    self.eat_ident_chars();
                }
            }
            None => {}
        }
    }

    /// `r"…"`, `r#"…"#`, `br…`, `cr…`, `b"…"`, `c"…"`, `b'…'` at an identifier start.
    fn prefixed_literal(&mut self) -> Option<Tok> {
        let line = self.line;
        let rest = &self.b[self.i..];
        let prefix = if rest.starts_with(b"br") || rest.starts_with(b"cr") {
            2
        } else if rest.starts_with(b"r") {
            1
        } else {
            0
        };
        if prefix > 0 {
            let hashes = rest[prefix..].iter().take_while(|&&c| c == b'#').count();
            if rest.get(prefix + hashes) == Some(&b'"') {
                self.i += prefix + hashes + 1;
                return Some(Tok::Str(line, self.raw(hashes)));
            }
        }
        if rest.starts_with(b"b\"") || rest.starts_with(b"c\"") {
            self.i += 2;
            return Some(Tok::Str(line, self.cooked()));
        }
        if rest.starts_with(b"b'") {
            self.i += 1;
            self.char_or_lifetime();
            return Some(Tok::Other);
        }
        None
    }
}

/// Splits Rust source into the tokens the extractor needs (comments dropped).
fn tokens(src: &str) -> Vec<Tok> {
    let mut lx = Lexer {
        src,
        b: src.as_bytes(),
        i: 0,
        line: 1,
    };
    let mut out = Vec::new();
    while let Some(c) = lx.peek(0) {
        match c {
            b'/' if lx.peek(1) == Some(b'/') => {
                while lx.peek(0).is_some_and(|c| c != b'\n') {
                    lx.next_char();
                }
            }
            b'/' if lx.peek(1) == Some(b'*') => lx.block_comment(),
            b'"' => {
                let line = lx.line;
                lx.i += 1;
                let text = lx.cooked();
                out.push(Tok::Str(line, text));
            }
            b'\'' => {
                lx.char_or_lifetime();
                out.push(Tok::Other);
            }
            c if c == b'_' || c.is_ascii_alphabetic() => {
                if let Some(tok) = lx.prefixed_literal() {
                    out.push(tok);
                    continue;
                }
                let start = lx.i;
                lx.eat_ident_chars();
                out.push(Tok::Ident(src[start..lx.i].to_owned()));
            }
            c if c.is_ascii_digit() => {
                lx.eat_ident_chars();
                out.push(Tok::Other);
            }
            c if c.is_ascii_whitespace() => {
                lx.next_char();
            }
            c if c.is_ascii() => {
                lx.i += 1;
                out.push(Tok::Punct(c));
            }
            _ => {
                lx.next_char();
                out.push(Tok::Other);
            }
        }
    }
    out
}

/// Index after `#[cfg(test)]` (or `#![cfg(test)]` when `inner`) starting at `i`.
fn cfg_test_at(toks: &[Tok], i: usize, inner: bool) -> Option<usize> {
    let punct = |j: usize, c: u8| toks.get(j) == Some(&Tok::Punct(c));
    let ident = |j: usize, s: &str| matches!(toks.get(j), Some(Tok::Ident(x)) if x == s);
    let mut j = i;
    if !punct(j, b'#') {
        return None;
    }
    j += 1;
    if inner {
        if !punct(j, b'!') {
            return None;
        }
        j += 1;
    }
    let shape = punct(j, b'[')
        && ident(j + 1, "cfg")
        && punct(j + 2, b'(')
        && ident(j + 3, "test")
        && punct(j + 4, b')')
        && punct(j + 5, b']');
    shape.then_some(j + 6)
}

/// What a `#[cfg(test)]` attribute applies to, judged from the code's first tokens: it decides
/// which tokens end that code in [`skip_item`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marked {
    /// An item that ends at its `;` or with its `{ … }` body: `fn`, `struct`, `enum`, `union`,
    /// `trait`, `impl`, `mod`, `macro_rules!` (after `pub` and qualifiers such as `const`,
    /// `async`, `unsafe`, `extern "C"`). A comma in its header (`impl<A, B>`) does not end it.
    Item,
    /// Code that runs to its `;`, braces included: `let`, `use`, `type`, `static`, `extern crate`
    /// (`let x = if a { … } else { … };`).
    Statement,
    /// `const NAME`: a const item, which runs to its `;` like a [`Marked::Statement`], or a
    /// const generic parameter, which the `>` of the parameter list ends.
    Const,
    /// Anything else, which a `,` also ends: a struct field, an enum variant, a match arm, a
    /// struct-literal field, an argument or element, a parameter, an expression statement.
    Element,
}

/// [`Marked`] kind of the code that starts at `j`.
fn marked(toks: &[Tok], mut j: usize) -> Marked {
    let ident = |k: usize| match toks.get(k) {
        Some(Tok::Ident(x)) => Some(x.as_str()),
        _ => None,
    };
    let punct = |k: usize, c: u8| toks.get(k) == Some(&Tok::Punct(c));
    if ident(j) == Some("pub") {
        j += 1;
        if punct(j, b'(') {
            while toks.get(j).is_some_and(|t| *t != Tok::Punct(b')')) {
                j += 1;
            }
            j += 1;
        }
    }
    loop {
        match ident(j) {
            // `const NAME` and `const _`, not `const fn`, `const unsafe fn` or `const { … }`.
            Some("const")
                if ident(j + 1)
                    .is_some_and(|n| !matches!(n, "fn" | "unsafe" | "extern" | "async")) =>
            {
                return Marked::Const;
            }
            Some("extern") if ident(j + 1) == Some("crate") => return Marked::Statement,
            Some("extern") if matches!(toks.get(j + 1), Some(Tok::Str(..))) => j += 2,
            Some("const" | "async" | "unsafe" | "safe" | "default" | "extern") => j += 1,
            _ => break,
        }
    }
    // A name follows (`static X`, `type X`, `union X`), not a closure (`static move ||`).
    let named = ident(j + 1).is_some_and(|n| n != "move");
    match ident(j) {
        Some("fn" | "struct" | "enum" | "trait" | "impl" | "mod") => Marked::Item,
        Some("union") if named => Marked::Item,
        Some("macro_rules") if punct(j + 1, b'!') => Marked::Item,
        Some("let") => Marked::Statement,
        Some("type" | "static") if named => Marked::Statement,
        Some("use") if named || punct(j + 1, b':') || punct(j + 1, b'{') => Marked::Statement,
        _ => Marked::Element,
    }
}

/// Index after the code that a `#[cfg(test)]` attribute applies to, which starts at `start`
/// (after the attributes).
///
/// The code ends at its `;`, at its `,` when it is an element of a list (a field, variant, match
/// arm, argument or parameter), or with its `{ … }` body: braces are counted only once a body
/// opens (from the start for a [`Marked::Statement`] or [`Marked::Const`], whose braces all come
/// before the `;`). It also ends, without consuming it, before a token that closes the
/// enclosing code: the `}`, `)` or `]` of the block or list it sits in, the `>` of a generic
/// parameter list, the `|` of closure parameters. The enclosing code is then scanned normally,
/// and the `{` positions [`scan`] tracks stay balanced. Where such a token is ambiguous (a `>`
/// comparison, the `|` of an or-pattern) it stops early: scanning some test code can only cause
/// a loud false alarm, while skipping too far would hide the classes that follow.
fn skip_item(toks: &[Tok], start: usize) -> usize {
    let kind = marked(toks, start);
    let to_semicolon = matches!(kind, Marked::Statement | Marked::Const);
    // The `>` of `->` or `=>`.
    let arrow = |j: usize| j > 0 && matches!(toks[j - 1], Tok::Punct(b'-' | b'='));
    // Open `(`/`[` (and `{` for code that runs to its `;`), and open `<` outside them.
    let (mut nesting, mut angles) = (0usize, 0usize);
    for (j, tok) in toks.iter().enumerate().skip(start) {
        let &Tok::Punct(c) = tok else {
            continue;
        };
        match c {
            b'(' | b'[' => nesting += 1,
            b'{' if to_semicolon => nesting += 1,
            // The body: skipped to its matching `}`.
            b'{' if nesting == 0 => return skip_block(toks, j),
            // The enclosing block or list closes.
            b'}' | b')' | b']' if nesting == 0 => return j,
            b')' | b']' => nesting -= 1,
            b'}' if to_semicolon => nesting -= 1,
            _ if nesting > 0 => {}
            b';' => return j + 1,
            b',' if kind == Marked::Element => return j + 1,
            b'|' if kind == Marked::Element => return j,
            b'<' => angles += 1,
            // Closes a `<`, or else the enclosing generic parameter list. A statement is never
            // in one, so there it is a comparison or a shift.
            b'>' if !arrow(j) => {
                if angles > 0 {
                    angles -= 1;
                } else if kind != Marked::Statement {
                    return j;
                }
            }
            _ => {}
        }
    }
    toks.len()
}

/// Index after the `}` matching the `{` at `j`.
fn skip_block(toks: &[Tok], mut j: usize) -> usize {
    let mut braces = 0usize;
    while j < toks.len() {
        match toks[j] {
            Tok::Punct(b'{') => braces += 1,
            Tok::Punct(b'}') => {
                braces = braces.saturating_sub(1);
                if braces == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
        j += 1;
    }
    j
}

/// Index after the outer attribute `#[…]` that starts at `i`.
fn attribute_end(toks: &[Tok], i: usize) -> Option<usize> {
    if toks.get(i) != Some(&Tok::Punct(b'#')) || toks.get(i + 1) != Some(&Tok::Punct(b'[')) {
        return None;
    }
    let mut depth = 0usize;
    for (j, tok) in toks.iter().enumerate().skip(i + 1) {
        match tok {
            Tok::Punct(b'[' | b'(' | b'{') => depth += 1,
            Tok::Punct(b']' | b')' | b'}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(j + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// The run of outer attributes starting at `i`: the index after it, whether one of them is
/// `#[cfg(test)]` and whether one is `#[path = …]`.
fn attributes_at(toks: &[Tok], i: usize) -> Option<(usize, bool, bool)> {
    let (mut start, mut end) = (i, attribute_end(toks, i)?);
    let (mut test, mut path) = (false, false);
    loop {
        test |= cfg_test_at(toks, start, false).is_some();
        path |= matches!(toks.get(start + 2), Some(Tok::Ident(x)) if x == "path")
            && toks.get(start + 3) == Some(&Tok::Punct(b'='));
        match attribute_end(toks, end) {
            Some(next) => (start, end) = (end, next),
            None => return Some((end, test, path)),
        }
    }
}

/// `[pub[(…)]] mod name` at `j`: the name and the index after it.
fn module_decl(toks: &[Tok], mut j: usize) -> Option<(String, usize)> {
    if matches!(toks.get(j), Some(Tok::Ident(x)) if x == "pub") {
        j += 1;
        if toks.get(j) == Some(&Tok::Punct(b'(')) {
            while toks.get(j).is_some_and(|t| *t != Tok::Punct(b')')) {
                j += 1;
            }
            j += 1;
        }
    }
    match (toks.get(j), toks.get(j + 1)) {
        (Some(Tok::Ident(kw)), Some(Tok::Ident(name))) if kw == "mod" => {
            Some((name.clone(), j + 2))
        }
        _ => None,
    }
}

/// The name of the inline module whose body opens at the `{` at `brace`.
fn inline_module_name(toks: &[Tok], brace: usize) -> Option<String> {
    let at = |k: Option<usize>| k.and_then(|k| toks.get(k));
    match (at(brace.checked_sub(2)), at(brace.checked_sub(1))) {
        (Some(Tok::Ident(kw)), Some(Tok::Ident(name))) if kw == "mod" => Some(name.clone()),
        _ => None,
    }
}

/// The inline modules around a point inside the blocks `open`; `None` inside any other block (a
/// function body, an `impl`), where no module file can be declared.
fn module_path(toks: &[Tok], open: &[usize]) -> Option<Vec<String>> {
    open.iter().map(|&b| inline_module_name(toks, b)).collect()
}

/// A module of test code whose descendants may live in their own files.
#[derive(Debug, PartialEq, Eq)]
struct TestModule {
    /// Path below the module directory of the file (enclosing inline modules, then the
    /// module's name); empty for the file's own module.
    path: Vec<String>,
    /// Declared out of line (`#[cfg(test)] mod name;`), so `name.rs` is test code too.
    out_of_line: bool,
}

/// What the extractor keeps of one source file.
#[derive(Debug, Default)]
struct Scan {
    /// String literals outside comments and test code, with the line each starts on.
    literals: Vec<(usize, String)>,
    /// Test modules whose files [`collect`] skips: `#[cfg(test)] mod name;`, and the
    /// `#[cfg(test)]` / `#![cfg(test)]` modules (and test files) whose children would have
    /// their own files. A test module with a `#[path]` attribute is not followed (its file is
    /// scanned like any other).
    test_modules: Vec<TestModule>,
}

/// String literals of `src` outside comments and `#[cfg(test)]` / `#![cfg(test)]` code, with
/// the line each starts on, and the test modules whose files are skipped.
fn scan(src: &str) -> Scan {
    let toks = tokens(src);
    let mut out = Vec::new();
    let mut test_modules = Vec::new();
    // Indices of the `{` of the blocks we are in.
    let mut open: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if cfg_test_at(&toks, i, true).is_some() {
            match open.pop() {
                // The whole file is test code, and so is every module below it.
                None => {
                    return Scan {
                        literals: Vec::new(),
                        test_modules: vec![TestModule {
                            path: Vec::new(),
                            out_of_line: false,
                        }],
                    };
                }
                // The enclosing module is test code: drop what it held so far and skip the rest.
                Some(start) => {
                    out.retain(|(_, _, at)| *at < start);
                    if let (Some(mut path), Some(name)) =
                        (module_path(&toks, &open), inline_module_name(&toks, start))
                    {
                        path.push(name);
                        test_modules.push(TestModule {
                            path,
                            out_of_line: false,
                        });
                    }
                    i = skip_block(&toks, start);
                    continue;
                }
            }
        }
        if let Some((end, true, path_attr)) = attributes_at(&toks, i) {
            // A `#[cfg(test)]` item (with the attributes around it). A test module keeps its
            // children's files out too.
            if let Some((name, after)) = module_decl(&toks, end) {
                let out_of_line = toks.get(after) == Some(&Tok::Punct(b';'));
                let inline = toks.get(after) == Some(&Tok::Punct(b'{'));
                if let Some(mut path) =
                    module_path(&toks, &open).filter(|_| !path_attr && (out_of_line || inline))
                {
                    path.push(name);
                    test_modules.push(TestModule { path, out_of_line });
                }
            }
            i = skip_item(&toks, end);
            continue;
        }
        match &toks[i] {
            Tok::Punct(b'{') => open.push(i),
            Tok::Punct(b'}') => {
                open.pop();
            }
            Tok::Str(line, text) => out.push((*line, text.clone(), i)),
            _ => {}
        }
        i += 1;
    }
    Scan {
        literals: out
            .into_iter()
            .map(|(line, text, _)| (line, text))
            .collect(),
        test_modules,
    }
}

// ---------------------------------------------------------------------------------------------
// Class tokens
// ---------------------------------------------------------------------------------------------

/// Candidate tokens of a literal with their line offset: whitespace separates them, and so do
/// quotes, `<`, `>` and `=` outside brackets (class lists inside HTML markup strings).
fn candidates(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let (mut line, mut depth) = (0usize, 0usize);
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        let separator =
            c.is_whitespace() || (depth == 0 && matches!(c, '"' | '\'' | '`' | '<' | '>' | '='));
        if separator {
            if let Some(s) = start.take() {
                out.push((line, &text[s..i]));
            }
            depth = 0;
            if c == '\n' {
                line += 1;
            }
            continue;
        }
        start.get_or_insert(i);
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((line, &text[s..]));
    }
    out
}

/// A leading variant: its length including the `:`, and whether it carries an arbitrary value.
///
/// Plain: `hover:`, `md:`, `2xl:`, `group-hover:`, `group-open/legacy:`, `@md:`, `*:`, `**:`.
/// Arbitrary: a selector with `&` or an at-rule in brackets (`[&::-webkit-details-marker]:`,
/// `[@supports(display:grid)]:`), and a variant with a bracketed value (`data-[state=open]:`,
/// `max-[600px]:`, `group-[.is-published]/item:`, `@min-[475px]:`, `@[17.5rem]:`,
/// `data-(--x):`).
fn variant(token: &str) -> Option<(usize, bool)> {
    let b = token.as_bytes();
    let stars = b.iter().take_while(|&&c| c == b'*').count();
    if stars > 0 {
        return (stars <= 2 && b.get(stars) == Some(&b':')).then_some((stars + 1, false));
    }
    if b.first() == Some(&b'[') {
        // Tailwind also takes a bare selector (`[p]:`, wrapped in `&:is(p)`), but so would
        // bracketed text such as `[SMILE]:`; only `&` selectors and at-rules count.
        let end = balanced_end(b, 0)?;
        let inner = &token[1..end];
        let ok = (inner.contains('&') || inner.starts_with('@')) && b.get(end + 1) == Some(&b':');
        return ok.then_some((end + 2, true));
    }
    let at = usize::from(b.first() == Some(&b'@'));
    let word = |c: Option<&u8>| c.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    // The name: `[a-z0-9]+(-[a-z0-9]+)*` after the optional `@`.
    let mut i = at;
    while word(b.get(i)) {
        i += 1;
        if b.get(i) == Some(&b'-') && word(b.get(i + 1)) {
            i += 1;
        }
    }
    let named = i > at;
    // A bracketed value after `name-`, or right after a bare `@` (`@[17.5rem]`).
    let open = match (named, b.get(i)) {
        (true, Some(b'-')) => Some(i + 1),
        (false, _) if at == 1 => Some(i),
        _ => None,
    };
    let mut arbitrary = false;
    if let Some(open) = open.filter(|&o| matches!(b.get(o), Some(b'[' | b'('))) {
        if b[open] == b'(' && !token[open + 1..].starts_with("--") {
            return None;
        }
        let end = balanced_end(b, open)?;
        if end == open + 1 {
            return None;
        }
        i = end + 1;
        arbitrary = true;
    } else if !named {
        return None;
    }
    // `/name`: named groups and peers (`group-hover/item:`, `group-open/legacy:`).
    if b.get(i) == Some(&b'/') {
        let start = i + 1;
        i = start;
        while b.get(i).is_some_and(|&c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_'
        }) {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    (b.get(i) == Some(&b':')).then_some((i + 1, arbitrary))
}

/// Index of the bracket that closes the one at `open` (`[]` and `()` both nest).
fn balanced_end(b: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (k, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'[' | b'(' => depth += 1,
            b']' | b')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// `/30`, `/7.5`, `/tight`, `/[20px]` after an arbitrary value.
fn modifier_ok(after: &str) -> bool {
    let Some(m) = after.strip_prefix('/') else {
        return false;
    };
    match m.as_bytes().first() {
        Some(b'[' | b'(') => balanced_end(m.as_bytes(), 0) == Some(m.len() - 1),
        Some(_) => m
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_')),
        None => false,
    }
}

/// The utility after the variants, `!` and `-`: `Some(true)` when it carries an arbitrary value
/// (`root-[value]` or `root-(--var)` with an optional `/modifier`, or a dashed utility with an
/// arbitrary modifier such as `text-sm/[17px]`), `Some(false)` for a plain utility (`hidden`,
/// `mt-2`, `w-1.5`, `bg-black/40`, `w-1/2`), `None` when it is no utility.
fn utility(rest: &str) -> Option<bool> {
    let b = rest.as_bytes();
    if !b.first().is_some_and(u8::is_ascii_lowercase) {
        return None;
    }
    let word = |c: Option<&u8>| c.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let mut i = 1;
    loop {
        while word(b.get(i)) {
            i += 1;
        }
        if b.get(i) == Some(&b'-') && word(b.get(i + 1)) {
            i += 1;
        } else {
            break;
        }
    }
    if b.get(i) == Some(&b'-') {
        let open = i + 1;
        match b.get(open) {
            Some(b'[') => {}
            Some(b'(') if rest[open + 1..].starts_with("--") => {}
            _ => return None,
        }
        let end = balanced_end(b, open)?;
        let after = &rest[end + 1..];
        return (end > open + 1 && (after.is_empty() || modifier_ok(after))).then_some(true);
    }
    // Plain: segments of `[a-z0-9.]` joined by single dashes, then an optional `/modifier`.
    let (base, modifier) = match rest.find('/') {
        Some(slash) => (&rest[..slash], Some(&rest[slash..])),
        None => (rest, None),
    };
    let plain = !base.ends_with('.')
        && base.split('-').all(|seg| {
            !seg.is_empty()
                && seg
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.')
        });
    if !plain {
        return None;
    }
    match modifier {
        None => Some(false),
        // An arbitrary modifier counts only on a dashed utility: `app/[lang]` is a path.
        Some(m) if matches!(m.as_bytes().get(1), Some(b'[' | b'(')) => {
            (base.contains('-') && modifier_ok(m)).then_some(true)
        }
        Some(m) => modifier_ok(m).then_some(false),
    }
}

/// True for a Tailwind class that carries an arbitrary value:
/// `[variant:]…[!][-]utility[/modifier][!]` where the utility (`hover:bg-[#d49e38]/10`,
/// `md:h-[calc(100vh-5rem)]`, `-left-[9px]`, `text-sm/[17px]`) or a variant
/// (`[&::-webkit-details-marker]:hidden`, `data-[state=open]:flex`) has one.
pub fn is_arbitrary_class(token: &str) -> bool {
    let mut rest = token;
    let mut arbitrary_variant = false;
    while let Some((len, arbitrary)) = variant(rest) {
        rest = &rest[len..];
        arbitrary_variant |= arbitrary;
    }
    let rest = rest.strip_prefix('!').unwrap_or(rest);
    let rest = rest.strip_suffix('!').unwrap_or(rest);
    let rest = rest.strip_prefix('-').unwrap_or(rest);
    utility(rest).is_some_and(|arbitrary| arbitrary || arbitrary_variant)
}

/// Arbitrary-value classes in Rust source: `(line, class)` in source order ([`collect`] runs
/// [`scan`] and [`classes_in_literals`] itself, because it also needs the test modules).
#[cfg(test)]
fn classes_in_source(src: &str) -> Vec<(usize, String)> {
    classes_in_literals(&scan(src).literals)
}

/// Arbitrary-value classes in extracted literals: `(line, class)` in order.
fn classes_in_literals(literals: &[(usize, String)]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (line, text) in literals {
        for (offset, token) in candidates(text) {
            if is_arbitrary_class(token) && !NOT_CLASSES.contains(&token) {
                out.push((line + offset, token.to_owned()));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// CSS
// ---------------------------------------------------------------------------------------------

/// CSSOM `CSS.escape`, which Tailwind 4 uses for class selectors (`.hover\:bg-\[\#d49e38\]\/10`).
pub fn css_escape(class: &str) -> String {
    let mut chars = class.chars();
    let first = chars.next();
    if first == Some('-') && chars.next().is_none() {
        return "\\-".to_owned();
    }
    let mut out = String::with_capacity(class.len() * 2);
    for (index, c) in class.chars().enumerate() {
        let code = u32::from(c);
        if c == '\0' {
            out.push('\u{FFFD}');
        } else if (0x01..=0x1f).contains(&code)
            || code == 0x7f
            || (index == 0 && c.is_ascii_digit())
            || (index == 1 && c.is_ascii_digit() && first == Some('-'))
        {
            out.push_str(&format!("\\{code:x} "));
        } else if code >= 0x80 || c == '-' || c == '_' || c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            out.push('\\');
            out.push(c);
        }
    }
    out
}

/// True when `css` has the class selector of `class`: `.` + [`css_escape`], where the `.` is not
/// itself escaped and the selector does not continue (so `.text-\[\#d49e38\]\/30` does not count
/// for `text-[#d49e38]`).
pub fn has_class_selector(css: &str, class: &str) -> bool {
    let needle = format!(".{}", css_escape(class));
    let mut from = 0;
    while let Some(pos) = css[from..].find(&needle) {
        let start = from + pos;
        let end = start + needle.len();
        let escaped_dot = css.as_bytes()[..start]
            .iter()
            .rev()
            .take_while(|&&b| b == b'\\')
            .count()
            % 2
            == 1;
        let continues = css[end..].chars().next().is_some_and(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '\\') || !c.is_ascii()
        });
        if !escaped_dot && !continues {
            return true;
        }
        from = start + 1;
    }
    false
}

// ---------------------------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------------------------

/// `.rs` files below `dir`, sorted, without `tests.rs` files and `tests/` directories.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        if entry.file_type()?.is_dir() {
            if name != "tests" {
                rust_files(&path, out)?;
            }
        } else if path.extension().is_some_and(|e| e == "rs") && name != "tests.rs" {
            out.push(path);
        }
    }
    Ok(())
}

/// The directory of the modules that `file` declares out of line (Rust's module-file rules):
/// the file's own directory for `mod.rs` and for the crate roots `lib.rs` / `main.rs` at the
/// top of the scanned directory `src_dir`, `<dir>/<stem>/` for every other file.
fn module_dir(src_dir: &Path, file: &Path) -> PathBuf {
    let dir = file.parent().unwrap_or(src_dir);
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if name == "mod.rs" || (dir == src_dir && matches!(name, "lib.rs" | "main.rs")) {
        dir.to_path_buf()
    } else {
        dir.join(file.file_stem().unwrap_or_default())
    }
}

/// Every arbitrary-value class in the `.rs` files of `dirs` (relative to `root`), skipping test
/// code: besides what [`rust_files`] leaves out, the files of test modules declared out of line
/// (`#[cfg(test)] mod boot_tests;` → `boot_tests.rs` or `boot_tests/mod.rs`) and every file
/// below a test module or test file (their child modules).
pub fn collect(root: &Path, dirs: &[&str]) -> Result<Classes> {
    let mut classes = Classes::new();
    for dir in dirs {
        let abs = root.join(dir);
        if !abs.is_dir() {
            bail!(
                "{} does not exist; update SOURCE_DIRS in xtask/src/classes.rs to the @source \
                 directories of apps/web/style/tailwind.css",
                abs.display()
            );
        }
        let mut files = Vec::new();
        rust_files(&abs, &mut files)?;
        let mut scans = Vec::with_capacity(files.len());
        for file in files {
            let src = fs::read_to_string(&file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            scans.push((file, scan(&src)));
        }
        // A declaring file may sort after the module's file, so resolve them all first.
        let (mut test_files, mut test_dirs) = (Vec::new(), Vec::new());
        for (file, found) in &scans {
            let base = module_dir(&abs, file);
            for module in &found.test_modules {
                let path = module.path.iter().fold(base.clone(), |p, seg| p.join(seg));
                if module.out_of_line {
                    test_files.push(path.with_extension("rs"));
                }
                test_dirs.push(path);
            }
        }
        for (file, found) in scans {
            if test_files.contains(&file) || test_dirs.iter().any(|d| file.starts_with(d)) {
                continue;
            }
            let shown = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            for (line, class) in classes_in_literals(&found.literals) {
                classes.entry(class).or_default().push(Use {
                    file: shown.clone(),
                    line,
                });
            }
        }
    }
    Ok(classes)
}

/// The `*.css` files directly in `dist`, sorted, and their concatenated text.
fn read_css(dist: &Path) -> Result<(Vec<PathBuf>, String)> {
    let entries = fs::read_dir(dist).with_context(|| {
        format!(
            "cannot read the dist directory {} (run the Trunk build first)",
            dist.display()
        )
    })?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "css"))
        .collect();
    files.sort();
    if files.is_empty() {
        bail!(
            "no .css file in {}: not a Trunk dist directory",
            dist.display()
        );
    }
    let mut css = String::new();
    for file in &files {
        css.push_str(
            &fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", file.display()))?,
        );
        css.push('\n');
    }
    Ok((files, css))
}

/// Checks the classes of `dirs` (below `root`) against the CSS files of `dist`.
pub fn check(root: &Path, dirs: &[&str], dist: &Path) -> Result<Outcome> {
    let classes = collect(root, dirs)?;
    if classes.is_empty() {
        bail!(
            "no Tailwind arbitrary-value class found in {}: the source list or the extractor is \
             out of date (xtask/src/classes.rs)",
            dirs.join(", ")
        );
    }
    let (css_files, css) = read_css(dist)?;
    let missing = classes
        .keys()
        .filter(|class| !has_class_selector(&css, class))
        .cloned()
        .collect();
    Ok(Outcome {
        classes,
        missing,
        css_files,
    })
}

/// The failure report for `outcome` (one entry per missing class, with every use).
fn report(outcome: &Outcome, dist: &Path) -> String {
    let mut msg = format!(
        "{} Tailwind arbitrary-value class(es) used in the sources are missing from the CSS in {}:",
        outcome.missing.len(),
        util::rel(dist)
    );
    for class in &outcome.missing {
        msg.push_str(&format!(
            "\n  {class}    (no selector .{})",
            css_escape(class)
        ));
        for u in outcome.classes.get(class).into_iter().flatten() {
            msg.push_str(&format!("\n      {}:{}", u.file, u.line));
        }
    }
    msg.push_str(
        "\nTailwind only generates a class it finds as one complete string literal with a value \
         it accepts: write the whole class literally (never build it with format! or concat!), \
         check the value's syntax, and keep the file under an @source of \
         apps/web/style/tailwind.css.",
    );
    msg
}

/// `cargo xtask check-classes`: checks [`SOURCE_DIRS`] against `dist`, prints a summary (every
/// class with `list`) and fails with the list of missing classes.
pub fn check_dist(dist: &Path, list: bool) -> Result<()> {
    let root = util::root();
    let outcome = check(&root, &SOURCE_DIRS, dist)?;
    let uses: usize = outcome.classes.values().map(Vec::len).sum();
    util::info(format!(
        "{} distinct arbitrary-value classes ({uses} uses) in {}",
        outcome.classes.len(),
        SOURCE_DIRS.join(", ")
    ));
    let shown: Vec<String> = outcome.css_files.iter().map(|f| util::rel(f)).collect();
    util::info(format!("searched {}", shown.join(", ")));
    if list {
        for (class, uses) in &outcome.classes {
            let state = if outcome.missing.contains(class) {
                "MISSING"
            } else {
                "ok     "
            };
            println!("{state} {class}  ({} uses)", uses.len());
        }
    }
    if outcome.missing.is_empty() {
        util::info("every class is in the built CSS");
        return Ok(());
    }
    for class in &outcome.missing {
        for u in outcome.classes.get(class).into_iter().flatten() {
            util::annotate_error(
                &u.file,
                u.line,
                &format!("Tailwind class `{class}` is missing from the built CSS"),
            );
        }
    }
    bail!("{}", report(&outcome, dist))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_like_tailwind() {
        for (class, escaped) in [
            ("text-[#d49e38]", r"text-\[\#d49e38\]"),
            ("border-[#d49e38]/20", r"border-\[\#d49e38\]\/20"),
            ("hover:bg-[#d49e38]/10", r"hover\:bg-\[\#d49e38\]\/10"),
            ("md:h-[calc(100vh-5rem)]", r"md\:h-\[calc\(100vh-5rem\)\]"),
            (
                "shadow-[0_0_15px_rgba(212,158,56,0.05)]",
                r"shadow-\[0_0_15px_rgba\(212\,158\,56\,0\.05\)\]",
            ),
            ("-left-[9px]", r"-left-\[9px\]"),
            ("bg-[size:4px_4px]", r"bg-\[size\:4px_4px\]"),
            (
                "group-open/legacy:w-[2px]",
                r"group-open\/legacy\:w-\[2px\]",
            ),
            ("w-[2px]!", r"w-\[2px\]\!"),
            // Arbitrary variants and modifiers (the selectors Tailwind 4.1.18 generates).
            (
                "[&::-webkit-details-marker]:hidden",
                r"\[\&\:\:-webkit-details-marker\]\:hidden",
            ),
            (
                "[@supports(display:grid)]:grid",
                r"\[\@supports\(display\:grid\)\]\:grid",
            ),
            (
                "data-[state=open]:bg-red-500",
                r"data-\[state\=open\]\:bg-red-500",
            ),
            (
                "group-data-[state=open]/item:w-1/2",
                r"group-data-\[state\=open\]\/item\:w-1\/2",
            ),
            ("@[17.5rem]:underline", r"\@\[17\.5rem\]\:underline"),
            ("text-sm/[17px]", r"text-sm\/\[17px\]"),
            ("2xl:text-[10px]", r"\32 xl\:text-\[10px\]"),
            ("-9", r"-\39 "),
            ("-", r"\-"),
            ("a\u{1}b", r"a\1 b"),
            ("é-[x]", "é-\\[x\\]"),
            ("a\0", "a\u{FFFD}"),
        ] {
            assert_eq!(css_escape(class), escaped, "{class}");
        }
    }

    #[test]
    fn recognises_arbitrary_value_classes_only() {
        for yes in [
            "text-[#d49e38]",
            "border-[#d49e38]/20",
            "text-[#d49e38]/90",
            "hover:bg-[#d49e38]",
            "md:h-[calc(100vh-5rem)]",
            "group-hover:translate-y-[2px]",
            "group-open/legacy:w-[2px]",
            "selection:bg-[#d49e38]",
            "marker:text-[#d49e38]",
            "2xl:min-w-[120px]",
            "@md:w-[2px]",
            "*:w-[2px]",
            "-left-[9px]",
            "!w-[2px]",
            "w-[2px]!",
            "bg-[size:4px_4px]",
            "bg-[linear-gradient(rgba(0,0,0,0)_0%,rgba(0,0,0,1)_90%)]",
            "text-[14px]/[20px]",
            "text-[14px]/7.5",
            "content-['x']",
            "bg-(--brand)",
            "w-[{w}px]",
            "w-[1px)",
        ] {
            assert!(is_arbitrary_class(yes), "{yes}");
        }
        for no in [
            "px-6",
            "text-sm",
            "bg-black/40",
            "w-1/2",
            "group-open/legacy:rotate-90",
            "hover:md:text-sm",
            "[SMILE]",
            "[mask-type:luminance]",
            "Array[0]",
            "foo[0]",
            "a-[",
            "text-[",
            "text-[]",
            "text-[10px]]",
            "text-[10px].",
            "text-[10px],",
            "text-[10px]/",
            "Text-[10px]",
            "https://x/a-[b]",
            "--x-[y]",
            "a--[x]",
            "bg-(brand)",
            "-[x]",
            "",
        ] {
            assert!(!is_arbitrary_class(no), "{no}");
        }
    }

    /// Arbitrary values in a variant or a modifier (every "yes" here generates CSS in Tailwind
    /// 4.1.18 except the typo'd `hiden`, which the guard must therefore report).
    #[test]
    fn recognises_arbitrary_variants_and_modifiers() {
        for yes in [
            // pages/download.rs (the legacy builds' <summary>) and its typo.
            "[&::-webkit-details-marker]:hidden",
            "[&::-webkit-details-marker]:hiden",
            "[&>p]:mt-2",
            "[&_p]:hover:underline",
            "md:[&>p]:mt-2",
            "[&>p]:!mt-2",
            "[&>p]:-mt-2",
            "[&>p]:mt-2!",
            "[&>p]:w-1.5",
            "[&>p]:bg-black/40",
            "[.dark_&]:text-[#d49e38]",
            "[@supports(display:grid)]:grid",
            "data-[state=open]:bg-red-500",
            "aria-[sort=ascending]:bg-black/40",
            "max-[600px]:hidden",
            "@min-[475px]:flex-row",
            "@[17.5rem]:underline",
            "group-[.is-published]:block",
            "group-data-[state=open]/item:w-1/2",
            "peer-[:checked]:text-[#d49e38]",
            "data-(--x):grid",
            // An arbitrary modifier on a plain utility.
            "text-sm/[17px]",
            "bg-red-500/[0.3]",
        ] {
            assert!(is_arbitrary_class(yes), "{yes}");
        }
        for no in [
            // Tailwind accepts these, but ordinary text has the same shapes: a bare-selector
            // variant (emotion tags such as `[SMILE]`) and arbitrary properties (log prefixes).
            "[SMILE]:hidden",
            "[p]:flex",
            "[&>p]:[mask-type:luminance]",
            // Broken variants, or no utility after them.
            "[&>p]:",
            "[&>p]mt-2",
            "[&>p:mt-2",
            "[&>p]:Mt-2",
            "[&>p]:mt-",
            "[&>p]:mt--2",
            "[&>p]:mt-2/",
            "[&>p]:mt-2.",
            "-[&>p]:mt-2",
            "data-[]:hidden",
            "data-(x):hidden",
            "data-[x:hidden",
            "-data-[x]:hidden",
            "data--[x]:hidden",
            "group-[.x]/:block",
            "***:hidden",
            // Paths with brackets.
            "app/[lang]",
            "src/[id]/page",
        ] {
            assert!(!is_arbitrary_class(no), "{no}");
        }
    }

    #[test]
    fn literals_skip_comments_chars_lifetimes_and_test_code() {
        let src = r####"
//! text-[#c1] in a doc comment
/* block /* nested text-[#c2] */ text-[#c3] */
fn f<'a>(x: &'a str) -> char { let q = '"'; let e = '\''; let u = '\u{1F600}'; 'x' }
const A: &str = "w-[1px] \"q\" line \
                 w-[2px]";
const B: &str = r#"w-[3px] "quoted" w-[4px]"#;
const C: &[u8] = b"w-[5px]";
const D: &str = r"w-[6px]";
const E: &str = "<span class='w-[7px] x'>y</span>";
#[cfg(test)]
mod tests {
    const T: &str = "w-[8px]";
    #[test]
    fn t() { let s = "w-[9px]"; }
}
#[cfg(test)]
const U: [&str; 1] = ["w-[10px]"];
#[cfg(test)]
use something;
mod inner {
    #![cfg(test)]
    const V: &str = "w-[11px]";
}
const G: &str = "w-[12px]"; // "w-[13px]"
"####;
        let classes: Vec<(usize, String)> = classes_in_source(src);
        let names: Vec<&str> = classes.iter().map(|(_, c)| c.as_str()).collect();
        assert_eq!(
            names,
            [
                "w-[1px]", "w-[2px]", "w-[3px]", "w-[4px]", "w-[5px]", "w-[6px]", "w-[7px]",
                "w-[12px]"
            ]
        );
        let lines: Vec<usize> = classes.iter().map(|(l, _)| *l).collect();
        assert_eq!(lines, [5, 5, 7, 7, 8, 9, 10, 25]);
        // `#![cfg(test)]` at the top: the whole file is test code.
        assert!(classes_in_source("#![cfg(test)]\nconst X: &str = \"w-[1px]\";").is_empty());
        // Multi-line literals report the line of each class.
        assert_eq!(
            classes_in_source("const X: &str = \"a\nb w-[1px]\n\nw-[2px]\";"),
            [(2, "w-[1px]".to_owned()), (4, "w-[2px]".to_owned())]
        );
    }

    fn test_module(path: &[&str], out_of_line: bool) -> TestModule {
        TestModule {
            path: path.iter().map(|s| (*s).to_owned()).collect(),
            out_of_line,
        }
    }

    #[test]
    fn test_module_declarations_are_recorded() {
        let src = r#"
#[cfg(test)]
mod boot_tests;
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod helpers;
#[allow(dead_code)]
#[cfg(test)]
pub mod fixtures;
mod real;
pub mod page;
mod outer {
    #[cfg(test)]
    mod inner;
    pub mod deeper {
        #[cfg(test)]
        mod t {
            const X: &str = "w-[1px]";
        }
        mod plain {
            #![cfg(test)]
            const Y: &str = "w-[2px]";
        }
    }
}
#[cfg(test)]
#[path = "elsewhere.rs"]
mod pathed;
#[path = "elsewhere2.rs"]
#[cfg(test)]
mod pathed2;
impl X {
    #[cfg(test)]
    const Z: &str = "w-[3px]";
}
fn f() {
    #[cfg(test)]
    mod in_block {}
}
#[cfg(test)]
use something;
#[doc = "w-[4px]"]
const KEPT: &str = "w-[5px]";
"#;
        let found = scan(src);
        let texts: Vec<&str> = found.literals.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(texts, ["w-[4px]", "w-[5px]"]);
        assert_eq!(
            found.test_modules,
            [
                test_module(&["boot_tests"], true),
                test_module(&["helpers"], true),
                test_module(&["fixtures"], true),
                test_module(&["outer", "inner"], true),
                test_module(&["outer", "deeper", "t"], false),
                test_module(&["outer", "deeper", "plain"], false),
            ]
        );
        // A test file: everything below its own module is test code.
        let file = scan("#![cfg(test)]\nmod child;\nconst X: &str = \"w-[1px]\";");
        assert!(file.literals.is_empty());
        assert_eq!(file.test_modules, [test_module(&[], false)]);
    }

    /// `#[cfg(test)]` skips only the code it marks, wherever stable Rust accepts it. Review
    /// finding: on a struct field, an enum variant or a match arm (code that ends in neither `;`
    /// nor a `{ … }` body) the skip ran past the enclosing `}` and silently dropped the literals
    /// of the next item. Every `w-` class here is outside test code; every `h-` class is inside.
    #[test]
    fn cfg_test_skips_only_the_code_it_marks() {
        let src = r#"
struct Probe {
    #[cfg(test)]
    only_in_tests: HashMap<u8, u8>,
    kept: u8,
}
fn probe_view() -> &'static str { "w-[1px]" }
struct Last { kept: u8, #[cfg(test)] only: u8 }
const AFTER_LAST_FIELD: &str = "w-[2px]";
struct Tuple(u8, #[cfg(test)] fn(u8, u8) -> u8);
const AFTER_TUPLE_FIELD: &str = "w-[3px]";
enum ProbeKind {
    #[cfg(test)]
    OnlyInTests,
    Real,
    #[cfg(test)]
    Braced { x: u8 },
    #[cfg(test)]
    Last = 9
}
const AFTER_VARIANTS: &str = "w-[4px]";
fn kind_class(k: ProbeKind) -> &'static str {
    match k {
        #[cfg(test)]
        ProbeKind::OnlyInTests => "h-[1px]",
        #[cfg(test)]
        ProbeKind::Last => { "h-[2px]" }
        ProbeKind::Real => "w-[5px]",
        #[cfg(test)]
        _ => "h-[3px]"
    }
}
const AFTER_ARMS: &str = "w-[6px]";
fn literal() -> Probe {
    Probe { #[cfg(test)] only_in_tests: HashMap::new(), kept: width("w-[7px]") }
}
fn args() -> (u8, [&'static str; 1]) {
    call(#[cfg(test)] "h-[4px]", "w-[8px]");
    (#[cfg(test)] 1, ["w-[9px]"], #[cfg(test)] "h-[5px]")
}
fn params(#[cfg(test)] fixture: &str) -> &'static str { "w-[10px]" }
fn closure() { let f = |a: u8, #[cfg(test)] b: u8| "w-[11px]"; }
fn generic<U, #[cfg(test)] T>() -> &'static str { "w-[12px]" }
fn const_generic<#[cfg(test)] const N: usize, U>() -> &'static str { "w-[13px]" }
fn statements() -> &'static str {
    #[cfg(test)]
    let fixture = if 2 > 1 { "h-[6px]" } else { "h-[7px]" };
    #[cfg(test)]
    check("h-[8px]");
    "w-[14px]"
}
#[cfg(test)]
impl<A, B> Helper<A, B> for Probe { fn f() -> &'static str { "h-[9px]" } }
#[cfg(test)]
fn helper() -> Result<(), String> { Err("h-[10px]".into()) }
#[cfg(test)]
pub(crate) async unsafe fn pair<A, B>(a: A, b: B) -> &'static str { "h-[11px]" }
#[cfg(test)]
pub const fn sized<A, B>() -> usize { 0 }
const AFTER_CONST_FN: &str = "w-[15px]";
#[cfg(test)]
const FIXTURE: HashMap<&str, &str> = from([("a", "h-[12px]")]);
#[cfg(test)]
static PICK: &str = if A > B { "h-[13px]" } else { "h-[14px]" };
#[cfg(test)]
use std::collections::{HashMap, HashSet};
#[cfg(test)]
pub(crate) struct Pair<A, B>(A, B, &'static str);
const END: &str = "w-[16px]";
"#;
        let names: Vec<String> = classes_in_source(src)
            .into_iter()
            .map(|(_, class)| class)
            .collect();
        let expected: Vec<String> = (1..=16).map(|n| format!("w-[{n}px]")).collect();
        assert_eq!(names, expected);

        // An ambiguous `|` (or-pattern) or `>` (comparison) ends the skip early: the marked arm
        // may then be scanned, but never the arms and items after it.
        let early = classes_in_source(
            r#"
fn f(k: u8) -> &'static str {
    match k {
        #[cfg(test)]
        1 | 2 => "h-[1px]",
        #[cfg(test)]
        n if n > 9 => "h-[2px]",
        _ => "w-[1px]",
    }
}
const AFTER: &str = "w-[2px]";
"#,
        );
        let kept: Vec<&str> = early
            .iter()
            .map(|(_, class)| class.as_str())
            .filter(|class| class.starts_with("w-"))
            .collect();
        assert_eq!(kept, ["w-[1px]", "w-[2px]"]);
    }

    /// The `{` positions [`scan`] tracks stay balanced around `#[cfg(test)]` fields, variants,
    /// arms and parameters, so later test module declarations are still recorded with their
    /// paths (the over-skip left the struct's `{` open, and nothing after it was recorded).
    #[test]
    fn module_tracking_survives_cfg_test_elements() {
        let found = scan(
            r#"
mod outer {
    struct S { #[cfg(test)] a: u8 }
    enum E { #[cfg(test)] A, B }
    fn f(e: E, #[cfg(test)] x: u8) -> u8 { match e { #[cfg(test)] E::A => 1, E::B => 2 } }
    #[cfg(test)]
    mod later;
}
#[cfg(test)]
mod top;
const K: &str = "w-[1px]";
"#,
        );
        assert_eq!(
            found.test_modules,
            [
                test_module(&["outer", "later"], true),
                test_module(&["top"], true)
            ]
        );
        let texts: Vec<&str> = found.literals.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(texts, ["w-[1px]"]);
    }

    /// The reviewer's case: a test module in its own file (`apps/web/src/boot_tests.rs`, declared
    /// as `#[cfg(test)] mod boot_tests;`) with a fixture class the CSS lacks.
    #[test]
    fn out_of_line_test_modules_are_skipped() {
        let tmp = std::env::temp_dir().join(format!("xtask-classes-mods-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let src = tmp.join("web/src");
        let files = [
            (
                "main.rs",
                "#[cfg(test)]\nmod boot_tests;\nmod boot_tests_extra;\nmod page;\nmod widgets;\n\
                 mod gen;\nmod outer {\n    #[cfg(test)]\n    mod inner;\n}\n#[cfg(test)]\n\
                 #[path = \"fx.rs\"]\nmod pathed;\nconst A: &str = \"text-[10px]\";\n",
            ),
            // Skipped: the test module, its child module, and the test modules of a non-mod-rs
            // file, a mod.rs file, an inline module and a test file.
            (
                "boot_tests.rs",
                "mod helpers;\nconst T: &str = \"w-[123px]\";\n",
            ),
            ("boot_tests/helpers.rs", "const H: &str = \"w-[124px]\";\n"),
            (
                "page.rs",
                "#[cfg(test)]\nmod fixtures;\nconst P: &str = \"-left-[9px]\";\n",
            ),
            ("page/fixtures.rs", "const F: &str = \"w-[126px]\";\n"),
            ("widgets/mod.rs", "mod tail;\n#[cfg(test)]\nmod t;\n"),
            ("widgets/t/mod.rs", "const W: &str = \"w-[128px]\";\n"),
            ("outer/inner.rs", "const I: &str = \"w-[127px]\";\n"),
            ("gen.rs", "#![cfg(test)]\nmod data;\n"),
            ("gen/data.rs", "const D: &str = \"w-[129px]\";\n"),
            // Scanned: a sibling whose name only starts like the test module, a normal child
            // module, and a test module behind `#[path]` (not followed).
            ("boot_tests_extra.rs", "const E: &str = \"w-[125px]\";\n"),
            ("widgets/tail.rs", "const L: &str = \"text-[10px]\";\n"),
            ("fx.rs", "const X: &str = \"w-[130px]\";\n"),
        ];
        for (name, text) in files {
            let path = src.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        let dist = tmp.join("dist");
        fs::create_dir_all(&dist).unwrap();
        fs::write(
            dist.join("tailwind.css"),
            r".text-\[10px\]{font-size:10px}.-left-\[9px\]{left:-9px}",
        )
        .unwrap();

        let outcome = check(&tmp, &["web/src"], &dist).unwrap();
        let found: Vec<(&str, Vec<&str>)> = outcome
            .classes
            .iter()
            .map(|(class, uses)| {
                (
                    class.as_str(),
                    uses.iter().map(|u| u.file.as_str()).collect(),
                )
            })
            .collect();
        assert_eq!(
            found,
            [
                ("-left-[9px]", vec!["web/src/page.rs"]),
                (
                    "text-[10px]",
                    vec!["web/src/main.rs", "web/src/widgets/tail.rs"]
                ),
                ("w-[125px]", vec!["web/src/boot_tests_extra.rs"]),
                ("w-[130px]", vec!["web/src/fx.rs"]),
            ]
        );
        assert_eq!(outcome.missing, ["w-[125px]", "w-[130px]"]);
        let _ = fs::remove_dir_all(&tmp);
    }

    /// The documented case on the real tree: `apps/web/src/main.rs` declares
    /// `#[cfg(test)] mod boot_tests;`, so a fixture class added to (a copy of) `boot_tests.rs` is
    /// not checked, while one added to `main.rs` is.
    #[test]
    fn the_real_boot_tests_module_is_skipped() {
        let web = util::root().join("apps/web/src");
        let tmp = std::env::temp_dir().join(format!("xtask-classes-web-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let copy = tmp.join("apps/web/src");
        fs::create_dir_all(&copy).unwrap();
        for (name, fixture) in [
            ("main.rs", "\nconst CONTROL: &str = \"w-[124px]\";\n"),
            (
                "boot_tests.rs",
                "\n#[test]\nfn fixture() {\n    let _ = \"<div class=\\\"w-[123px]\\\"></div>\";\n}\n",
            ),
        ] {
            let text = fs::read_to_string(web.join(name)).unwrap();
            fs::write(copy.join(name), text + fixture).unwrap();
        }
        let main = fs::read_to_string(copy.join("main.rs")).unwrap();
        assert!(scan(&main)
            .test_modules
            .contains(&test_module(&["boot_tests"], true)));

        let classes = collect(&tmp, &["apps/web/src"]).unwrap();
        assert!(
            !classes.contains_key("w-[123px]"),
            "boot_tests.rs was scanned"
        );
        let control: Vec<&str> = classes["w-[124px]"]
            .iter()
            .map(|u| u.file.as_str())
            .collect();
        assert_eq!(control, ["apps/web/src/main.rs"]);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn selector_lookup_respects_boundaries() {
        let css = r".text-\[\#d49e38\]\/30{color:red}.py-0\.text-\[10px\]{}.a,.hover\:bg-\[\#fff\]:hover{}";
        assert!(has_class_selector(css, "text-[#d49e38]/30"));
        // A longer selector that starts with the class does not count.
        assert!(!has_class_selector(css, "text-[#d49e38]"));
        // An escaped dot is not a class start.
        assert!(!has_class_selector(css, "text-[10px]"));
        assert!(has_class_selector(css, "hover:bg-[#fff]"));
        assert!(has_class_selector(".x-\\[1\\]", "x-[1]"));
    }

    /// The deliberately broken fixture: tokens Tailwind could not have generated (a typo'd
    /// colour and a typo'd utility after an arbitrary variant, both absent from the CSS) and one
    /// assembled with `format!`.
    const FIXTURE_RS: &str = r#"
const CARD: &str = "px-6 border border-[#d49e38]/40 hover:bg-[#d49e38] text-[10px]";
fn view() -> impl IntoView {
    view! { <div class="md:h-[calc(100vh-5rem)] text-[#d49e3]">"Press Enter to send"</div> }
}
fn dynamic(w: u32) -> String {
    format!("w-[{w}px] -left-[9px]")
}
fn details() -> impl IntoView {
    view! { <summary class="list-none [&::-webkit-details-marker]:hidden">"[SMILE]: [p]:flex"</summary> }
}
const TYPO: &str = "cursor-pointer [&::-webkit-details-marker]:hiden";
"#;
    const FIXTURE_CSS: &str = r".border-\[\#d49e38\]\/40{border-color:#d49e3866}@media (hover:hover){.hover\:bg-\[\#d49e38\]:hover{background:#d49e38}}.text-\[10px\]{font-size:10px}@media (min-width:48rem){.md\:h-\[calc\(100vh-5rem\)\]{height:calc(100vh - 5rem)}}.-left-\[9px\]{left:-9px}.\[\&\:\:-webkit-details-marker\]\:hidden::-webkit-details-marker{display:none}";

    #[test]
    fn broken_fixture_reports_exactly_the_missing_tokens() {
        let tmp = std::env::temp_dir().join(format!("xtask-classes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let src = tmp.join("ui/src");
        fs::create_dir_all(src.join("pages")).unwrap();
        fs::create_dir_all(src.join("tests")).unwrap();
        fs::write(src.join("pages/card.rs"), FIXTURE_RS).unwrap();
        // Test code is not scanned.
        fs::write(src.join("tests.rs"), "const X: &str = \"w-[99px]\";").unwrap();
        fs::write(src.join("tests/x.rs"), "const X: &str = \"w-[98px]\";").unwrap();
        let dist = tmp.join("dist");
        fs::create_dir_all(&dist).unwrap();
        fs::write(dist.join("tailwind-0123.css"), FIXTURE_CSS).unwrap();
        fs::write(dist.join("index.html"), "<base href=/amadeus/>").unwrap();

        let outcome = check(&tmp, &["ui/src"], &dist).unwrap();
        assert_eq!(outcome.classes.len(), 9);
        assert_eq!(
            outcome.missing,
            [
                "[&::-webkit-details-marker]:hiden",
                "text-[#d49e3]",
                "w-[{w}px]"
            ]
        );
        assert_eq!(
            outcome.classes["text-[#d49e3]"],
            [Use {
                file: "ui/src/pages/card.rs".into(),
                line: 4
            }]
        );
        assert_eq!(
            outcome.classes["[&::-webkit-details-marker]:hiden"],
            [Use {
                file: "ui/src/pages/card.rs".into(),
                line: 12
            }]
        );
        assert_eq!(
            outcome.classes["[&::-webkit-details-marker]:hidden"],
            [Use {
                file: "ui/src/pages/card.rs".into(),
                line: 10
            }]
        );
        assert_eq!(outcome.css_files, [dist.join("tailwind-0123.css")]);
        let msg = report(&outcome, &dist);
        assert!(
            msg.starts_with("3 Tailwind arbitrary-value class(es)"),
            "{msg}"
        );
        assert!(msg.contains("\n  text-[#d49e3]    (no selector .text-\\[\\#d49e3\\])\n      ui/src/pages/card.rs:4"), "{msg}");
        assert!(
            msg.contains(
                "\n  w-[{w}px]    (no selector .w-\\[\\{w\\}px\\])\n      ui/src/pages/card.rs:7"
            ),
            "{msg}"
        );
        assert!(
            msg.contains(
                "\n  [&::-webkit-details-marker]:hiden    (no selector \
                 .\\[\\&\\:\\:-webkit-details-marker\\]\\:hiden)\n      ui/src/pages/card.rs:12"
            ),
            "{msg}"
        );

        // With the classes fixed the check passes.
        fs::write(
            src.join("pages/card.rs"),
            FIXTURE_RS
                .replace("text-[#d49e3]", "text-[10px]")
                .replace("w-[{w}px] ", "")
                .replace(":hiden", ":hidden"),
        )
        .unwrap();
        assert!(check(&tmp, &["ui/src"], &dist).unwrap().missing.is_empty());

        // A dist without CSS, a missing source directory or no classes at all is an error.
        let empty = tmp.join("empty");
        fs::create_dir_all(&empty).unwrap();
        assert!(check(&tmp, &["ui/src"], &empty).is_err());
        assert!(check(&tmp, &["nope"], &dist).is_err());
        fs::create_dir_all(tmp.join("plain")).unwrap();
        fs::write(tmp.join("plain/a.rs"), "const A: &str = \"px-6\";").unwrap();
        assert!(check(&tmp, &["plain"], &dist).is_err());
        let _ = fs::remove_dir_all(&tmp);
    }

    /// The review's repro: `#[cfg(test)]` on a struct field, an enum variant and a match arm,
    /// each followed by code with a class the CSS lacks (`h-[1px]` sits in the marked arm).
    const ELEMENTS_RS: &str = r#"
struct Probe {
    #[cfg(test)]
    only_in_tests: u8,
    kept: u8,
}
fn probe_view() -> &'static str { "flex w-[778px]" }
enum ProbeKind {
    #[cfg(test)]
    OnlyInTests,
    Real,
}
const AFTER_VARIANT: &str = "flex w-[779px]";
fn kind_class(k: ProbeKind) -> &'static str {
    match k {
        #[cfg(test)]
        ProbeKind::OnlyInTests => "h-[1px]",
        ProbeKind::Real => "flex w-[780px]",
    }
}
const AFTER_ARM: &str = "flex w-[781px] w-[1px]";
"#;

    #[test]
    fn classes_after_cfg_test_elements_are_reported() {
        let tmp =
            std::env::temp_dir().join(format!("xtask-classes-elements-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let pages = tmp.join("ui/src/pages");
        fs::create_dir_all(&pages).unwrap();
        fs::write(pages.join("probe.rs"), ELEMENTS_RS).unwrap();
        let dist = tmp.join("dist");
        fs::create_dir_all(&dist).unwrap();
        fs::write(dist.join("tailwind.css"), r".w-\[1px\]{width:1px}").unwrap();

        let outcome = check(&tmp, &["ui/src"], &dist).unwrap();
        let found: Vec<(&str, Vec<usize>)> = outcome
            .classes
            .iter()
            .map(|(class, uses)| (class.as_str(), uses.iter().map(|u| u.line).collect()))
            .collect();
        assert_eq!(
            found,
            [
                ("w-[1px]", vec![21]),
                ("w-[778px]", vec![7]),
                ("w-[779px]", vec![13]),
                ("w-[780px]", vec![18]),
                ("w-[781px]", vec![21]),
            ]
        );
        assert_eq!(
            outcome.missing,
            ["w-[778px]", "w-[779px]", "w-[780px]", "w-[781px]"]
        );
        let msg = report(&outcome, &dist);
        assert!(
            msg.contains(
                "\n  w-[778px]    (no selector .w-\\[778px\\])\n      ui/src/pages/probe.rs:7"
            ),
            "{msg}"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn source_dirs_are_the_rust_sources_of_tailwind_css() {
        let root = util::root();
        let css = fs::read_to_string(root.join("apps/web/style/tailwind.css")).unwrap();
        let mut globs: Vec<String> = css
            .lines()
            .filter_map(|l| l.trim().strip_prefix("@source \""))
            .filter_map(|l| l.strip_suffix("/**/*.rs\";"))
            .map(|rel| {
                // Relative to apps/web/style.
                let mut parts: Vec<&str> = vec!["apps", "web", "style"];
                for seg in rel.split('/') {
                    match seg {
                        ".." => {
                            parts.pop();
                        }
                        "." => {}
                        s => parts.push(s),
                    }
                }
                parts.join("/")
            })
            .collect();
        globs.sort();
        let mut dirs: Vec<String> = SOURCE_DIRS.iter().map(|d| (*d).to_owned()).collect();
        dirs.sort();
        assert_eq!(globs, dirs);
    }

    #[test]
    fn the_real_sources_have_the_known_classes() {
        let classes = collect(&util::root(), &SOURCE_DIRS).unwrap();
        for known in [
            "text-[#d49e38]",
            "border-[#d49e38]/20",
            "hover:bg-[#d49e38]",
            "md:h-[calc(100vh-5rem)]",
            "-left-[9px]",
            // An arbitrary variant (pages/download.rs, the legacy builds' <summary>).
            "[&::-webkit-details-marker]:hidden",
        ] {
            assert!(classes.contains_key(known), "{known} not found");
        }
        assert!(classes.keys().all(|c| is_arbitrary_class(c)));
    }
}
