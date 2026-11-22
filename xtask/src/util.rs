//! Small cross-platform helpers: workspace paths, running commands, files, hashes, dates.

use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read, Write as _},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

/// Product name used for every release asset (`RealAmadeus_V2.0_...`).
pub const PRODUCT: &str = "RealAmadeus";

/// Absolute path of the workspace root (the directory that holds the top-level `Cargo.toml`).
pub fn root() -> PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        // `cargo run` sets CARGO_MANIFEST_DIR at runtime; fall back to the compile-time value.
        let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        let manifest_dir = manifest_dir.canonicalize().unwrap_or(manifest_dir);
        let root = manifest_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(manifest_dir);
        // `canonicalize` on Windows yields `\\?\C:\...`, which some tools (trunk, tar) dislike.
        strip_verbatim(root)
    })
    .clone()
}

fn strip_verbatim(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        if let Some(s) = path.to_str() {
            if let Some(rest) = s.strip_prefix(r"\\?\") {
                if !rest.starts_with("UNC\\") {
                    return PathBuf::from(rest);
                }
            }
        }
    }
    path
}

/// `release/` — where every distributable ends up.
pub fn release_dir() -> PathBuf {
    root().join("release")
}

/// Path relative to the workspace root, for messages (`apps/web/dist`, not `/home/.../apps/web/dist`).
pub fn rel(path: &Path) -> String {
    let root = root();
    let shown = path.strip_prefix(&root).unwrap_or(path);
    let s = shown.display().to_string();
    if s.is_empty() {
        ".".to_string()
    } else {
        s
    }
}

/// True when running inside GitHub Actions (enables log groups / annotations / step summary).
pub fn in_github_actions() -> bool {
    env::var_os("GITHUB_ACTIONS").is_some_and(|v| v == "true")
}

/// True on CI systems (GitHub Actions and most others set `CI=true`).
pub fn in_ci() -> bool {
    env::var("CI").is_ok_and(|v| v == "true" || v == "1")
}

/// Value of an environment variable, treating "set but empty" as unset
/// (GitHub Actions expands missing secrets to empty strings).
pub fn env_non_empty(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

// ---------------------------------------------------------------------------------------------
// Console output. Everything human-oriented (progress, warnings and GitHub workflow commands,
// which the runner also reads from stderr) goes to stderr so stdout stays clean for
// machine-readable output (`cargo xtask version`, `cargo xtask release-notes`).
// ---------------------------------------------------------------------------------------------

pub fn step(msg: impl AsRef<str>) {
    eprintln!("\n==> {}", msg.as_ref());
}

pub fn info(msg: impl AsRef<str>) {
    eprintln!("    {}", msg.as_ref());
}

pub fn warn(msg: impl AsRef<str>) {
    let msg = msg.as_ref();
    if in_github_actions() {
        // Workflow command: shows up as an annotation on the run summary.
        eprintln!("::warning::{}", escape_workflow_data(msg));
    } else {
        eprintln!("warning: {msg}");
    }
}

fn escape_workflow_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// GitHub Actions error annotation on a source line (`file` relative to the repository root);
/// no-op elsewhere.
pub fn annotate_error(file: &str, line: usize, msg: &str) {
    if in_github_actions() {
        let file = escape_workflow_data(file)
            .replace(':', "%3A")
            .replace(',', "%2C");
        eprintln!(
            "::error file={file},line={line}::{}",
            escape_workflow_data(msg)
        );
    }
}

/// Appends Markdown to the GitHub Actions job summary (no-op elsewhere).
pub fn job_summary(markdown: &str) {
    let Some(path) = env::var_os("GITHUB_STEP_SUMMARY") else {
        return;
    };
    let result = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(markdown.as_bytes()));
    if let Err(err) = result {
        warn(format!("could not write the job summary: {err}"));
    }
}

// ---------------------------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------------------------

/// A `Command` that starts in the workspace root and does not leak the `cargo run`
/// environment of xtask itself (CARGO_PKG_*, CARGO_MANIFEST_*) into child processes.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(root());
    for (key, _) in env::vars_os() {
        let Some(key) = key.to_str() else { continue };
        if key.starts_with("CARGO_PKG_")
            || matches!(
                key,
                "CARGO_MANIFEST_DIR"
                    | "CARGO_MANIFEST_PATH"
                    | "CARGO_CRATE_NAME"
                    | "CARGO_BIN_NAME"
                    | "CARGO_PRIMARY_PACKAGE"
            )
        {
            cmd.env_remove(key);
        }
    }
    cmd
}

/// The cargo that runs xtask (`$CARGO`), so the same toolchain is used throughout.
pub fn cargo() -> Command {
    command(env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")))
}

/// Human readable form of a command line, e.g. `(in apps/web) trunk build --release`.
pub fn describe(cmd: &Command) -> String {
    let mut parts = vec![display_program(cmd.get_program())];
    parts.extend(cmd.get_args().map(|a| quote(&a.to_string_lossy())));
    let line = parts.join(" ");
    match cmd.get_current_dir() {
        Some(dir) if dir != root() => format!("(in {}) {line}", rel(dir)),
        _ => line,
    }
}

fn display_program(program: &OsStr) -> String {
    let path = Path::new(program);
    // Show `cargo` rather than `/home/me/.rustup/toolchains/.../bin/cargo`.
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| program.to_string_lossy().into_owned());
    if name == "cargo" {
        name
    } else {
        program.to_string_lossy().into_owned()
    }
}

fn quote(arg: &str) -> String {
    if arg.is_empty() || arg.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
        format!("{arg:?}")
    } else {
        arg.to_string()
    }
}

fn program_name(cmd: &Command) -> String {
    Path::new(cmd.get_program())
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Installation hint shown when a program is missing.
fn install_hint(program: &str) -> &'static str {
    match program {
        "trunk" => {
            "Install Trunk 0.22: `cargo install --locked trunk --version 0.22.1` \
             (or a prebuilt binary from https://github.com/trunk-rs/trunk/releases)."
        }
        "git" => "Install Git and make sure `git` is on PATH.",
        "tar" => {
            "A `tar` executable is required to build archives \
             (Windows 10+ ships one as %SystemRoot%\\System32\\tar.exe)."
        }
        "zip" => "Install Info-ZIP `zip` (e.g. `sudo apt install zip`) or libarchive's `bsdtar`.",
        "rustc" | "cargo" | "rustup" => "Install Rust via https://rustup.rs.",
        _ => "",
    }
}

fn spawn_error(cmd: &Command, err: io::Error) -> anyhow::Error {
    let program = program_name(cmd);
    if err.kind() == io::ErrorKind::NotFound {
        anyhow!(
            "`{program}` was not found on PATH. {}",
            install_hint(&program)
        )
    } else {
        anyhow!(err).context(format!("failed to start `{}`", describe(cmd)))
    }
}

fn status_text(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit code {code}"),
        None => "terminated by a signal".to_string(),
    }
}

/// Runs a command with inherited stdio (output streams live) and fails on a non-zero exit.
pub fn run(cmd: &mut Command) -> Result<()> {
    let shown = describe(cmd);
    let grouped = in_github_actions();
    if grouped {
        eprintln!("::group::{shown}");
    } else {
        eprintln!("$ {shown}");
    }
    let status = cmd.status();
    if grouped {
        eprintln!("::endgroup::");
    }
    let status = status.map_err(|e| spawn_error(cmd, e))?;
    if !status.success() {
        bail!("command failed ({}): {shown}", status_text(status));
    }
    Ok(())
}

/// Runs a command and returns its stdout (stderr is passed through).
pub fn capture(cmd: &mut Command) -> Result<String> {
    let shown = describe(cmd);
    let output = cmd
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| spawn_error(cmd, e))?;
    if !output.status.success() {
        bail!("command failed ({}): {shown}", status_text(output.status));
    }
    String::from_utf8(output.stdout).with_context(|| format!("non UTF-8 output from `{shown}`"))
}

/// Like [`capture`] but silent: `None` when the program is missing or fails.
pub fn probe(cmd: &mut Command) -> Option<String> {
    let output = cmd
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

/// True when `program --version` (or the given probe arguments) runs successfully.
pub fn has_program(program: impl AsRef<OsStr>, args: &[&str]) -> bool {
    probe(command(program).args(args)).is_some()
}

/// Host target triple as reported by `rustc -vV`.
pub fn host_triple() -> Result<String> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let out = capture(command(rustc).arg("-vV"))?;
    out.lines()
        .find_map(|l| l.strip_prefix("host: "))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow!("could not determine the host triple from `rustc -vV`"))
}

/// Cargo's target directory (honours CARGO_TARGET_DIR and `build.target-dir`).
pub fn target_dir() -> Result<PathBuf> {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    if let Some(dir) = DIR.get() {
        return Ok(dir.clone());
    }
    let out = capture(cargo().args(["metadata", "--format-version", "1", "--no-deps"]))?;
    let meta: serde_json::Value =
        serde_json::from_str(&out).context("could not parse `cargo metadata` output")?;
    let dir = meta
        .get("target_directory")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("`cargo metadata` did not report a target_directory"))?;
    Ok(DIR.get_or_init(|| strip_verbatim(dir)).clone())
}

/// Scratch space for xtask inside the target directory (`target/xtask`).
pub fn scratch_dir() -> Result<PathBuf> {
    Ok(target_dir()?.join("xtask"))
}

// ---------------------------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------------------------

pub fn remove_dir_if_exists(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

pub fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("failed to remove {}", path.display())),
    }
}

/// Recursively copies `src` into `dst`, skipping entries for which `skip(name)` is true.
pub fn copy_dir_all(src: &Path, dst: &Path, skip: &dyn Fn(&OsStr) -> bool) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| format!("failed to create {}", dst.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("failed to read {}", src.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        if skip(&name) {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        // `metadata` follows symlinks, so linked files/dirs are copied by content.
        let meta =
            fs::metadata(&from).with_context(|| format!("failed to stat {}", from.display()))?;
        if meta.is_dir() {
            copy_dir_all(&from, &to, skip)?;
        } else {
            fs::copy(&from, &to).with_context(|| {
                format!("failed to copy {} -> {}", from.display(), to.display())
            })?;
        }
    }
    Ok(())
}

pub fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::copy(from, to)
        .with_context(|| format!("failed to copy {} -> {}", from.display(), to.display()))?;
    Ok(())
}

pub fn read_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to read {}", rel(path)))
}

pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", rel(path)))
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

// ---------------------------------------------------------------------------------------------
// SHA-256
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
pub fn sha256_hex(data: &[u8]) -> String {
    to_hex(&Sha256::digest(data))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(to_hex(&hasher.finalize()))
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// Writes `<file>.sha256` in `sha256sum` format (`<hex>  <name>`) and returns the hash.
pub fn write_sha256_file(path: &Path) -> Result<String> {
    let hash = sha256_file(path)?;
    let name = path
        .file_name()
        .ok_or_else(|| anyhow!("{} has no file name", path.display()))?
        .to_string_lossy();
    let mut sidecar = path.as_os_str().to_owned();
    sidecar.push(".sha256");
    fs::write(PathBuf::from(sidecar), format!("{hash}  {name}\n"))?;
    Ok(hash)
}

/// Files in `release/` that are distributables (not checksums / signatures / hidden files).
pub fn is_distributable(name: &str) -> bool {
    !(name.starts_with('.')
        || name.ends_with(".sha256")
        || name.ends_with(".sig")
        || name == SHA256SUMS
        || name == "latest.json")
}

pub const SHA256SUMS: &str = "SHA256SUMS.txt";

/// Rewrites `release/SHA256SUMS.txt` for every distributable currently in `release/`.
pub fn update_sha256sums(release: &Path) -> Result<PathBuf> {
    let mut names: Vec<String> = fs::read_dir(release)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| is_distributable(n))
        .collect();
    names.sort();
    let mut out = String::new();
    for name in &names {
        out.push_str(&format!("{}  {name}\n", sha256_file(&release.join(name))?));
    }
    let path = release.join(SHA256SUMS);
    fs::write(&path, out)?;
    Ok(path)
}

// ---------------------------------------------------------------------------------------------
// Dates (no chrono needed for a YYYY-MM-DD stamp)
// ---------------------------------------------------------------------------------------------

/// Today's date in UTC, `YYYY-MM-DD`.
pub fn today_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Today's date in the local time zone (asks the OS), falling back to UTC.
pub fn today_local() -> String {
    let local = if cfg!(windows) {
        probe(command("powershell").args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-Date -Format yyyy-MM-dd",
        ]))
    } else {
        probe(command("date").arg("+%Y-%m-%d"))
    };
    local
        .map(|s| s.trim().to_string())
        .filter(|s| is_iso_date(s))
        .unwrap_or_else(today_utc)
}

/// Days since 1970-01-01 -> (year, month, day). Howard Hinnant's `civil_from_days`.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Validates a `YYYY-MM-DD` string.
pub fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && (1..=12).contains(&s[5..7].parse::<u32>().unwrap_or(0))
        && (1..=31).contains(&s[8..10].parse::<u32>().unwrap_or(0))
}

/// Replaces `{{key}}` placeholders in a template.
pub fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(20_773), (2026, 11, 16));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn iso_dates() {
        assert!(is_iso_date("2026-11-16"));
        assert!(!is_iso_date("2026-13-01"));
        assert!(!is_iso_date("2026/11/16"));
        assert!(!is_iso_date("26-11-16"));
        assert!(is_iso_date(&today_utc()));
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn templates() {
        assert_eq!(
            render("V{{version}}R / {{version}}", &[("version", "2.0.0")]),
            "V2.0.0R / 2.0.0"
        );
    }

    #[test]
    fn distributables() {
        assert!(is_distributable("RealAmadeus_V2.0_Linux_amd64.deb"));
        assert!(!is_distributable("RealAmadeus_V2.0_Linux_amd64.deb.sha256"));
        assert!(!is_distributable(
            "RealAmadeus_V2.0_Linux_amd64.AppImage.sig"
        ));
        assert!(!is_distributable(SHA256SUMS));
        assert!(!is_distributable(".staging"));
    }

    #[test]
    fn workflow_data_is_escaped() {
        assert_eq!(escape_workflow_data("a%b\nc"), "a%25b%0Ac");
    }

    #[test]
    fn sizes() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1536), "1.5 KiB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MiB");
    }
}
