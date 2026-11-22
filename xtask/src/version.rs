//! The release version: `[workspace.package] version` in the workspace `Cargo.toml` is the
//! single source of truth (every crate inherits it, Tauri reads it from apps/desktop/Cargo.toml).

use std::{cmp::Ordering, fmt, path::PathBuf, str::FromStr};

use anyhow::{anyhow, bail, Context, Result};

use crate::{changelog, util};

// ---------------------------------------------------------------------------------------------
// SemVer 2.0.0 (just enough for release tooling; no extra crate needed)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// Pre-release identifiers (`rc.1` -> ["rc", "1"]). Empty for a normal release.
    pub pre: Vec<String>,
    /// Build metadata (ignored for precedence).
    pub build: Option<String>,
}

impl Version {
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    /// Git tag of this version: `v2.0.0`.
    pub fn tag(&self) -> String {
        format!("v{self}")
    }

    /// Product label: `V2.0` (drops `.0` patch on release versions).
    pub fn label(&self) -> String {
        if self.patch == 0 && self.pre.is_empty() && self.build.is_none() {
            format!("V{}.{}", self.major, self.minor)
        } else {
            format!("V{self}")
        }
    }

    /// Same version, ignoring build metadata.
    pub fn same_release(&self, other: &Version) -> bool {
        self.cmp_precedence(other) == Ordering::Equal
    }

    /// Whether Tauri can build a WiX `.msi` for this version. MSI versions are
    /// `major.minor.patch[.build]` with major/minor <= 255 and patch/build <= 65535, so the
    /// bundler only accepts a single numeric pre-release identifier (`2.1.0-1`) or numeric build
    /// metadata; `2.1.0-rc.1` fails. NSIS installers accept any SemVer pre-release.
    pub fn msi_compatible(&self) -> bool {
        let small = |s: &str| s.parse::<u64>().is_ok_and(|n| n <= 65_535);
        let pre_ok = match self.pre.as_slice() {
            [] => true,
            [only] => small(only),
            _ => false,
        };
        let build_ok = self.build.as_deref().is_none_or(small);
        self.major <= 255 && self.minor <= 255 && self.patch <= 65_535 && pre_ok && build_ok
    }

    /// SemVer precedence (build metadata is ignored).
    pub fn cmp_precedence(&self, other: &Version) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                // A pre-release has lower precedence than the normal version.
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => cmp_pre(&self.pre, &other.pre),
            })
    }
}

/// Format a version string for release labels and asset prefixes (`2.0.0` → `V2.0`).
pub fn format_label(version: &str) -> String {
    if !version.contains('-') && !version.contains('+') {
        let mut parts = version.split('.');
        if let (Some(major), Some(minor), Some("0"), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        {
            return format!("V{major}.{minor}");
        }
    }
    format!("V{version}")
}

fn cmp_pre(a: &[String], b: &[String]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a.len().cmp(&b.len())
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{build}")?;
        }
        Ok(())
    }
}

impl FromStr for Version {
    type Err = anyhow::Error;

    /// Accepts `2.0.1`, `v2.0.1`, `V2.0.1`, legacy `V2.0.1R`, and SemVer pre-release/build suffixes.
    fn from_str(input: &str) -> Result<Self> {
        let err = || {
            anyhow!(
                "`{input}` is not a valid version (expected MAJOR.MINOR.PATCH, e.g. 2.0.1 or 2.1.0-rc.1)"
            )
        };
        let mut s = input.trim();
        s = s
            .strip_prefix('v')
            .or_else(|| s.strip_prefix('V'))
            .unwrap_or(s);
        // Optional legacy product label form `V2.0.1R`: drop a trailing `'R'` after a digit.
        if let Some(rest) = s.strip_suffix('R') {
            if rest.ends_with(|c: char| c.is_ascii_digit()) {
                s = rest;
            }
        }

        let (s, build) = match s.split_once('+') {
            Some((v, b)) => (v, Some(b)),
            None => (s, None),
        };
        let (core, pre) = match s.split_once('-') {
            Some((v, p)) => (v, Some(p)),
            None => (s, None),
        };

        let nums: Vec<&str> = core.split('.').collect();
        if nums.len() != 3 {
            return Err(err());
        }
        let num = |p: &str| -> Result<u64> {
            if p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
            {
                return Err(err());
            }
            p.parse::<u64>().map_err(|_| err())
        };

        let ident_ok =
            |id: &str| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        let pre: Vec<String> = match pre {
            None => Vec::new(),
            Some(p) => {
                let ids: Vec<String> = p.split('.').map(str::to_string).collect();
                for id in &ids {
                    let numeric = id.bytes().all(|b| b.is_ascii_digit());
                    if !ident_ok(id) || (numeric && id.len() > 1 && id.starts_with('0')) {
                        return Err(err());
                    }
                }
                ids
            }
        };
        if let Some(b) = build {
            if !b.split('.').all(ident_ok) {
                return Err(err());
            }
        }

        Ok(Version {
            major: num(nums[0])?,
            minor: num(nums[1])?,
            patch: num(nums[2])?,
            pre,
            build: build.map(str::to_string),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Workspace Cargo.toml
// ---------------------------------------------------------------------------------------------

pub fn manifest_path() -> PathBuf {
    util::root().join("Cargo.toml")
}

/// Name of a TOML table header line (`[workspace.package]` -> `workspace.package`).
fn table_header(line: &str) -> Option<String> {
    let t = line.trim();
    if !t.starts_with('[') || t.starts_with("[[") {
        return None;
    }
    let end = t.find(']')?;
    Some(t[1..end].split_whitespace().collect::<String>())
}

/// Locates `version = "..."` inside `[workspace.package]`: (line index, byte range of the value).
fn find_version_line(manifest: &str) -> Option<(usize, std::ops::Range<usize>)> {
    find_string_key(manifest, "workspace.package", "version")
}

/// Locates `key = "..."` inside `[table]`: (line index, byte range of the string value).
fn find_string_key(
    manifest: &str,
    table: &str,
    key: &str,
) -> Option<(usize, std::ops::Range<usize>)> {
    let mut in_section = false;
    for (idx, line) in manifest.lines().enumerate() {
        if let Some(name) = table_header(line) {
            in_section = name == table;
            continue;
        }
        if line.trim_start().starts_with("[[") {
            in_section = false;
            continue;
        }
        if !in_section {
            continue;
        }
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix(key) else {
            continue;
        };
        let rest_trim = rest.trim_start();
        let Some(after_eq) = rest_trim.strip_prefix('=') else {
            continue;
        };
        let after_eq_trim = after_eq.trim_start();
        let Some(value_and_tail) = after_eq_trim.strip_prefix('"') else {
            continue;
        };
        let close = value_and_tail.find('"')?;
        let start = line.len() - value_and_tail.len();
        return Some((idx, start..start + close));
    }
    None
}

/// The raw version string of the workspace.
pub fn read_workspace_version() -> Result<String> {
    let path = manifest_path();
    let manifest = util::read_to_string(&path)?;
    let (idx, range) = find_version_line(&manifest).ok_or_else(|| {
        anyhow!(
            "could not find `version = \"...\"` in the [workspace.package] table of {}",
            util::rel(&path)
        )
    })?;
    let line = manifest.lines().nth(idx).unwrap_or_default();
    Ok(line[range].to_string())
}

/// `[workspace.package] repository` (the GitHub URL of the V2 repository).
pub fn workspace_repository() -> Result<String> {
    let manifest = util::read_to_string(&manifest_path())?;
    let (idx, range) = find_string_key(&manifest, "workspace.package", "repository")
        .ok_or_else(|| anyhow!("no `repository` in [workspace.package]"))?;
    Ok(manifest.lines().nth(idx).unwrap_or_default()[range].to_string())
}

/// The parsed workspace version.
pub fn current() -> Result<Version> {
    let raw = read_workspace_version()?;
    raw.parse().with_context(|| {
        format!(
            "invalid [workspace.package] version in {}",
            util::rel(&manifest_path())
        )
    })
}

/// Rewrites only the version value, keeping every other byte of the file. Returns the new text.
fn replace_version(manifest: &str, new: &str) -> Option<String> {
    let (idx, range) = find_version_line(manifest)?;
    let mut out = String::with_capacity(manifest.len() + 8);
    // `split_inclusive` keeps the original line endings (`\n` or `\r\n`).
    for (i, line) in manifest.split_inclusive('\n').enumerate() {
        if i == idx {
            out.push_str(&line[..range.start]);
            out.push_str(new);
            out.push_str(&line[range.end..]);
        } else {
            out.push_str(line);
        }
    }
    Some(out)
}

fn set_workspace_version(new: &Version) -> Result<bool> {
    let path = manifest_path();
    let manifest = util::read_to_string(&path)?;
    let updated = replace_version(&manifest, &new.to_string()).ok_or_else(|| {
        anyhow!(
            "could not locate the workspace version in {}",
            util::rel(&path)
        )
    })?;
    if updated == manifest {
        return Ok(false);
    }
    util::write(&path, updated)?;
    Ok(true)
}

/// Refreshes the workspace members' entries in Cargo.lock without touching dependencies.
fn update_lockfile() -> Result<()> {
    util::step("Updating Cargo.lock (workspace members only)");
    let offline = util::run(util::cargo().args(["update", "--workspace", "--offline"]));
    if let Err(err) = offline {
        util::warn(format!(
            "offline lockfile update failed ({err:#}); retrying online"
        ));
        util::run(util::cargo().args(["update", "--workspace"]))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// `cargo xtask version`
// ---------------------------------------------------------------------------------------------

pub enum VersionFormat {
    Plain,
    Tag,
    Label,
    Json,
}

pub fn print(format: VersionFormat) -> Result<()> {
    let v = current()?;
    match format {
        VersionFormat::Plain => println!("{v}"),
        VersionFormat::Tag => println!("{}", v.tag()),
        VersionFormat::Label => println!("{}", v.label()),
        VersionFormat::Json => println!(
            "{}",
            serde_json::json!({
                "version": v.to_string(),
                "tag": v.tag(),
                "label": v.label(),
                "prerelease": v.is_prerelease(),
                "msi_compatible": v.msi_compatible(),
                "asset_prefix": format!("{}_{}", util::PRODUCT, v.label()),
            })
        ),
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// `cargo xtask bump`
// ---------------------------------------------------------------------------------------------

pub struct BumpOptions {
    pub version: String,
    pub commit: bool,
    pub tag: bool,
    pub date: Option<String>,
    pub allow_downgrade: bool,
}

fn git() -> std::process::Command {
    util::command("git")
}

/// The files a release commit consists of (paths relative to the workspace root).
const RELEASE_FILES: [&str; 3] = ["Cargo.toml", "Cargo.lock", "CHANGELOG.md"];

fn ensure_git_repo() -> Result<()> {
    let inside = util::capture(git().args(["rev-parse", "--is-inside-work-tree"]))
        .context("`--commit`/`--tag` need a Git repository")?;
    if inside.trim() != "true" {
        bail!("{} is not inside a Git work tree", util::root().display());
    }
    // Otherwise `--commit` would create a commit holding only the release files, e.g. when this
    // workspace is still an untracked directory inside a bigger repository.
    if util::probe(git().args(["ls-files", "--error-unmatch", "--", "Cargo.toml"])).is_none() {
        bail!(
            "the workspace is not committed to Git yet (Cargo.toml is untracked). Commit it \
             first; see docs/RELEASE.md §1.1"
        );
    }
    Ok(())
}

/// True when `file` (relative to the workspace root) differs from HEAD or is untracked.
fn is_dirty(file: &str) -> bool {
    util::probe(git().args(["status", "--porcelain", "--", file]))
        .is_some_and(|s| !s.trim().is_empty())
}

fn dirty_release_files() -> Vec<&'static str> {
    RELEASE_FILES.into_iter().filter(|f| is_dirty(f)).collect()
}

fn tag_exists(tag: &str) -> bool {
    util::probe(git().args(["rev-parse", "-q", "--verify", &format!("refs/tags/{tag}")])).is_some()
}

pub fn bump(opts: BumpOptions) -> Result<()> {
    let new: Version = opts.version.parse()?;
    let current = current()?;
    let date = match opts.date {
        Some(d) if util::is_iso_date(&d) => d,
        Some(d) => bail!("--date must be YYYY-MM-DD, got `{d}`"),
        None => util::today_local(),
    };

    match new.cmp_precedence(&current) {
        Ordering::Less if !opts.allow_downgrade => bail!(
            "{new} is lower than the current version {current}. Pass --allow-downgrade if that is intended."
        ),
        Ordering::Equal if new.build == current.build => {
            util::info(format!("The workspace is already at {new}; only CHANGELOG.md / Git steps run."))
        }
        _ => {}
    }

    let changelog_path = changelog::path();
    let changelog_has_section = changelog::has_section(&changelog_path, &new)?;
    let will_change_files =
        !new.same_release(&current) || new.build != current.build || !changelog_has_section;

    // Validate every Git precondition before touching any file.
    if opts.commit || opts.tag {
        ensure_git_repo()?;
    }
    if opts.tag {
        if tag_exists(&new.tag()) {
            bail!("Git tag {} already exists", new.tag());
        }
        if !opts.commit {
            if will_change_files {
                bail!(
                    "`--tag` without `--commit` would tag a commit that does not contain the version bump; \
                     use `cargo xtask bump {new} --commit --tag`"
                );
            }
            let dirty = dirty_release_files();
            if !dirty.is_empty() {
                bail!(
                    "{} differ(s) from HEAD, so the tag would not contain those changes; commit them \
                     first or use `cargo xtask bump {new} --commit --tag`",
                    dirty.join(", ")
                );
            }
        }
        // The CHANGELOG section becomes the GitHub release body.
        if !changelog::notes_ready(&changelog_path, &new, &date)? {
            bail!(
                "CHANGELOG.md has no release notes for {new}, so the tagged release would have an \
                 empty body. Write them under `## [Unreleased]` (bump moves them into [{new}]) and \
                 run this again, or bump without `--tag`, edit the new section, commit and run \
                 `cargo xtask bump {new} --tag`."
            );
        }
    }

    let mut changed: Vec<&str> = Vec::new();

    util::step(format!("Setting the workspace version to {new}"));
    if set_workspace_version(&new)? {
        util::info(format!(
            "{}: {current} -> {new}",
            util::rel(&manifest_path())
        ));
        update_lockfile()?;
        changed.extend(["Cargo.toml", "Cargo.lock"]);
    } else {
        util::info("Cargo.toml already has this version");
    }

    match changelog::ensure_section(&changelog_path, &new, &date)? {
        changelog::Ensured::Existing => util::info(format!(
            "{} already has a section for {new}",
            util::rel(&changelog_path)
        )),
        changelog::Ensured::FromUnreleased => {
            util::info(format!(
                "{}: moved the [Unreleased] notes into [{new}] - {date}",
                util::rel(&changelog_path)
            ));
            changed.push("CHANGELOG.md");
        }
        changelog::Ensured::Template => {
            util::warn(format!(
                "{}: added an empty [{new}] section — write the release notes before tagging",
                util::rel(&changelog_path)
            ));
            changed.push("CHANGELOG.md");
        }
    }

    if opts.commit {
        // Everything that differs from HEAD, including edits made before this run (release
        // notes written by hand, a bump done earlier without --commit).
        let to_commit = dirty_release_files();
        if to_commit.is_empty() {
            util::info("Nothing to commit (files already up to date)");
        } else {
            util::step("Committing the release");
            let message = format!("Release {} {}", util::PRODUCT, new.label());
            // Pathspecs limit the commit to these files even if other changes are staged.
            util::run(git().args(["add", "--"]).args(&to_commit))?;
            util::run(
                git()
                    .args(["commit", "-m", &message, "--"])
                    .args(&to_commit),
            )?;
        }
    }

    if opts.tag {
        util::step(format!("Tagging {}", new.tag()));
        let message = format!("{} {}", util::PRODUCT, new.label());
        // Annotated, so `git push --follow-tags` picks it up.
        util::run(git().args(["tag", "-a", &new.tag(), "-m", &message]))?;
    }

    util::step("Next steps");
    if !opts.commit && !changed.is_empty() {
        util::info(format!(
            "Review and commit: git commit -m \"Release {} {}\" -- {}",
            util::PRODUCT,
            new.label(),
            changed.join(" ")
        ));
    }
    if opts.tag {
        util::info("Push the commit and the tag (this starts the release workflow):");
        util::info("  git push --follow-tags");
    } else {
        util::info(format!(
            "Tag and push to release: git tag -a {0} -m \"{1} {2}\" && git push --follow-tags",
            new.tag(),
            util::PRODUCT,
            new.label()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        s.parse().unwrap()
    }

    #[test]
    fn parses_versions() {
        assert_eq!(v("2.0.1").to_string(), "2.0.1");
        assert_eq!(v("v2.0.1").to_string(), "2.0.1");
        assert_eq!(v("V2.0.1").to_string(), "2.0.1");
        assert_eq!(v("V2.0.1R").to_string(), "2.0.1");
        assert_eq!(v("2.1.0-rc.1").to_string(), "2.1.0-rc.1");
        assert_eq!(v("2.1.0-rc.1+build.5").to_string(), "2.1.0-rc.1+build.5");
        assert!(v("2.1.0-beta").is_prerelease());
        assert!(!v("2.1.0").is_prerelease());
        assert_eq!(v("2.0.1").tag(), "v2.0.1");
        assert_eq!(v("2.0.0").label(), "V2.0");
        assert_eq!(v("2.1.0").label(), "V2.1");
        assert_eq!(v("2.0.1").label(), "V2.0.1");
        assert_eq!(v("2.1.0-rc.1").label(), "V2.1.0-rc.1");
    }

    #[test]
    fn rejects_invalid_versions() {
        for bad in [
            "",
            "2",
            "2.0",
            "2.0.0.0",
            "02.0.0",
            "2.0.x",
            "2.0.0-",
            "2.0.0-01",
            "2.0.0+",
            "2.0.0-a..b",
        ] {
            assert!(
                bad.parse::<Version>().is_err(),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn precedence() {
        let order = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
            "2.0.0",
            "2.0.1",
            "2.1.0",
            "10.0.0",
        ];
        for pair in order.windows(2) {
            assert_eq!(
                v(pair[0]).cmp_precedence(&v(pair[1])),
                Ordering::Less,
                "{pair:?}"
            );
        }
        assert!(v("2.0.0+a").same_release(&v("2.0.0+b")));
    }

    #[test]
    fn msi_version_rules() {
        for ok in [
            "2.0.0",
            "2.1.0-1",
            "2.1.0-65535",
            "2.0.0+7",
            "255.255.65535",
        ] {
            assert!(v(ok).msi_compatible(), "{ok} should be MSI compatible");
        }
        for bad in [
            "2.1.0-rc.1",
            "2.1.0-beta",
            "2.1.0-1.2",
            "2.1.0-65536",
            "2.0.0+build.5",
            "256.0.0",
            "2.256.0",
            "2.0.65536",
        ] {
            assert!(
                !v(bad).msi_compatible(),
                "{bad} should not be MSI compatible"
            );
        }
    }

    const MANIFEST: &str = "[workspace]\r\nmembers = [\"a\"]\r\n\r\n[workspace.package]\r\nversion = \"2.0.0\" # release\r\nedition = \"2021\"\r\n\r\n[package]\r\nversion = \"9.9.9\"\r\n";

    #[test]
    fn finds_and_replaces_only_the_workspace_version() {
        let (idx, range) = find_version_line(MANIFEST).unwrap();
        let line = MANIFEST.lines().nth(idx).unwrap();
        assert_eq!(&line[range], "2.0.0");

        let out = replace_version(MANIFEST, "2.0.1").unwrap();
        assert_eq!(
            out,
            MANIFEST.replace("\"2.0.0\" # release", "\"2.0.1\" # release")
        );
        assert!(out.contains("version = \"9.9.9\""));
        assert!(out.contains("\r\n"));
    }

    #[test]
    fn ignores_versions_outside_the_table() {
        let text = "[package]\nversion = \"1.0.0\"\n[workspace.dependencies]\nversion = \"3\"\n";
        assert!(find_version_line(text).is_none());
    }

    #[test]
    fn reads_the_real_manifest() {
        let raw = read_workspace_version().unwrap();
        raw.parse::<Version>().unwrap();
    }
}
