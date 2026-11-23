//! CHANGELOG.md handling ("Keep a Changelog" layout):
//!
//! ```text
//! ## [Unreleased]
//!
//! ## [2.0.1] - 2026-12-01
//! ### Fixed
//! - ...
//! ```
//!
//! `release-notes` prints one section (the GitHub release body), `bump` adds the section of a
//! new version (moving the `[Unreleased]` notes into it when there are any).

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::{
    platform::{self, Bundle, Target},
    util,
    version::{self, Version},
};

/// Text inserted into a new, empty section. `release-notes` warns while it is still there.
pub const PLACEHOLDER: &str =
    "(ここに変更内容を書いてください / Describe the changes in this release)";

/// Public URL of the hosted web version.
pub const WEB_URL: &str = "https://dsuv.elvelt.com/amadeus";

pub fn path() -> PathBuf {
    util::root().join("CHANGELOG.md")
}

#[derive(Debug, PartialEq)]
enum Heading {
    Unreleased,
    Version(Version),
    Other,
}

#[derive(Debug)]
struct Section {
    heading: Heading,
    /// Index of the `## ` heading line.
    start: usize,
    /// Index one past the last line of the section.
    end: usize,
}

fn classify(line: &str) -> Option<Heading> {
    let rest = line.strip_prefix("## ")?.trim();
    let token = match rest.strip_prefix('[') {
        Some(inner) => inner.split(']').next().unwrap_or_default(),
        None => rest.split_whitespace().next().unwrap_or_default(),
    };
    Some(if token.eq_ignore_ascii_case("unreleased") {
        Heading::Unreleased
    } else if let Ok(v) = token.parse::<Version>() {
        Heading::Version(v)
    } else {
        Heading::Other
    })
}

fn sections(lines: &[&str]) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let is_h1 = line.starts_with("# ");
        let heading = classify(line);
        if heading.is_some() || is_h1 {
            if let Some(last) = out.last_mut() {
                if last.end == usize::MAX {
                    last.end = idx;
                }
            }
        }
        if let Some(heading) = heading {
            out.push(Section {
                heading,
                start: idx,
                end: usize::MAX,
            });
        }
    }
    if let Some(last) = out.last_mut() {
        if last.end == usize::MAX {
            last.end = lines.len();
        }
    }
    out
}

/// `[2.0.1]: https://...` style link reference definitions.
fn is_link_ref(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('[')
        && t.find("]:")
            .is_some_and(|i| i > 1 && !t[1..i].contains(']'))
}

fn is_noise(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || is_link_ref(t) || (t.starts_with("<!--") && t.ends_with("-->"))
}

fn body(lines: &[&str], section: &Section) -> String {
    let mut body: Vec<&str> = lines[section.start + 1..section.end].to_vec();
    while body
        .last()
        .is_some_and(|l| l.trim().is_empty() || is_link_ref(l))
    {
        body.pop();
    }
    let first = body
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(body.len());
    body[first..].join("\n")
}

fn find<'a>(sections: &'a [Section], version: &Version) -> Option<&'a Section> {
    sections
        .iter()
        .find(|s| matches!(&s.heading, Heading::Version(v) if v.same_release(version)))
}

pub fn has_section(path: &Path, version: &Version) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let text = util::read_to_string(path)?;
    let lines: Vec<&str> = text.lines().collect();
    Ok(find(&sections(&lines), version).is_some())
}

/// The body of the section for `version` (without its heading).
pub fn section_body(text: &str, version: &Version) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let secs = sections(&lines);
    find(&secs, version).map(|s| body(&lines, s))
}

pub fn release_notes(version: &Version) -> Result<String> {
    let path = path();
    if !path.exists() {
        bail!("{} does not exist", util::rel(&path));
    }
    let text = util::read_to_string(&path)?;
    let Some(notes) = section_body(&text, version) else {
        bail!(
            "{} has no section for {version}. Add a `## [{version}] - YYYY-MM-DD` heading \
             (or run `cargo xtask bump {version}`).",
            util::rel(&path)
        );
    };
    if notes.trim().is_empty() {
        util::warn(format!("the CHANGELOG section for {version} is empty"));
    } else if notes.contains(PLACEHOLDER) {
        util::warn(format!(
            "the CHANGELOG section for {version} still contains the placeholder text"
        ));
    }
    Ok(notes)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Ensured {
    /// The section already existed; nothing changed.
    Existing,
    /// The `[Unreleased]` notes became the new section.
    FromUnreleased,
    /// An empty template section was inserted.
    Template,
}

const HEADER: &str = "# Changelog / 変更履歴\n\n\
All notable changes to RealAmadeus V2 (Rust) are documented here. \
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the \
project follows [Semantic Versioning](https://semver.org/).\n\n## [Unreleased]\n";

/// True when a section body holds real release notes (not empty, not the template text).
fn has_real_notes(body: &str) -> bool {
    !body.contains(PLACEHOLDER) && body.lines().any(|l| !is_noise(l))
}

/// Whether the section for `version` will contain real notes once [`ensure_section`] has run:
/// either it already exists with content, or `[Unreleased]` has notes that move into it.
/// Nothing is written.
pub fn notes_ready(path: &Path, version: &Version, date: &str) -> Result<bool> {
    let text = if path.exists() {
        util::read_to_string(path)?
    } else {
        HEADER.to_string()
    };
    let (updated, _) = insert_section(&text, version, date);
    Ok(section_body(&updated, version).is_some_and(|b| has_real_notes(&b)))
}

/// Makes sure CHANGELOG.md has a section for `version`.
pub fn ensure_section(path: &Path, version: &Version, date: &str) -> Result<Ensured> {
    let text = if path.exists() {
        util::read_to_string(path)?
    } else {
        HEADER.to_string()
    };
    let (updated, how) = insert_section(&text, version, date);
    if how != Ensured::Existing {
        util::write(path, updated)?;
    }
    Ok(how)
}

fn insert_section(text: &str, version: &Version, date: &str) -> (String, Ensured) {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.lines().collect();
    let secs = sections(&lines);
    if find(&secs, version).is_some() {
        return (text.to_string(), Ensured::Existing);
    }
    let heading = format!("## [{version}] - {date}");
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();

    let unreleased = secs.iter().find(|s| s.heading == Heading::Unreleased);
    let has_notes =
        unreleased.is_some_and(|s| lines[s.start + 1..s.end].iter().any(|l| !is_noise(l)));

    let how = if let (Some(u), true) = (unreleased, has_notes) {
        // Keep an empty [Unreleased] on top; its notes now belong to the new version.
        out.splice(u.start + 1..u.start + 1, [String::new(), heading]);
        Ensured::FromUnreleased
    } else {
        let at = unreleased
            .map(|u| u.end)
            .or_else(|| {
                secs.iter()
                    .find(|s| matches!(s.heading, Heading::Version(_)))
                    .map(|s| s.start)
            })
            .unwrap_or_else(|| {
                // Before the trailing block of link reference definitions, if any.
                let mut at = out.len();
                while at > 0 && is_noise(&out[at - 1]) {
                    at -= 1;
                }
                if at == 0 {
                    out.len()
                } else {
                    at
                }
            });
        let mut block = Vec::new();
        if at > 0 && !out[at - 1].trim().is_empty() {
            block.push(String::new());
        }
        block.extend([
            heading,
            String::new(),
            "### 変更 / Changed".to_string(),
            String::new(),
            format!("- {PLACEHOLDER}"),
            String::new(),
        ]);
        out.splice(at..at, block);
        Ensured::Template
    };

    update_compare_links(&mut out, version);

    let mut joined = out.join(nl);
    if text.ends_with('\n') || text.is_empty() {
        joined.push_str(nl);
    }
    (joined, how)
}

/// Keeps `[Unreleased]: .../compare/vX...HEAD` links current, if the file uses them.
fn update_compare_links(lines: &mut Vec<String>, version: &Version) {
    let Some(idx) = lines.iter().position(|l| {
        l.trim_start()
            .to_ascii_lowercase()
            .starts_with("[unreleased]:")
    }) else {
        return;
    };
    let line = lines[idx].clone();
    let url = line
        .split_once("]:")
        .map(|x| x.1.trim())
        .unwrap_or_default();
    let Some((base, range)) = url.split_once("/compare/") else {
        return;
    };
    let Some(prev) = range.strip_suffix("...HEAD") else {
        return;
    };
    let tag = version.tag();
    if prev == tag {
        return;
    }
    let own_key = format!("[{version}]:");
    let has_own = lines.iter().any(|l| l.trim_start().starts_with(&own_key));
    lines[idx] = format!("[Unreleased]: {base}/compare/{tag}...HEAD");
    if !has_own {
        lines.insert(
            idx + 1,
            format!("[{version}]: {base}/compare/{prev}...{tag}"),
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Download guide appended to the GitHub release body
// ---------------------------------------------------------------------------------------------

/// `owner/repo` of the GitHub repository: `GITHUB_REPOSITORY` on CI, else the workspace
/// `repository` URL in Cargo.toml.
pub fn repository_slug() -> String {
    if let Some(slug) = util::env_non_empty("GITHUB_REPOSITORY") {
        return slug;
    }
    version::workspace_repository()
        .ok()
        .and_then(|url| {
            url.trim_end_matches('/')
                .trim_end_matches(".git")
                .split("github.com/")
                .nth(1)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "dsuv-sg/RealAmadeus".to_string())
}

pub fn download_guide(version: &Version) -> String {
    let slug = repository_slug();
    let owner = slug
        .split('/')
        .next()
        .unwrap_or("dsuv-sg")
        .to_ascii_lowercase();
    let tag = version.tag();
    let link = |file: &str| {
        format!("[`{file}`](https://github.com/{slug}/releases/download/{tag}/{file})")
    };
    let v = version.to_string();

    let win = Target::parse("x86_64-pc-windows-msvc").expect("static triple");
    let mac = Target::parse("universal-apple-darwin").expect("static triple");
    let linux = Target::parse("x86_64-unknown-linux-gnu").expect("static triple");
    let name = |t: &Target, b: Bundle| platform::bundle_asset_name(&v, t, b, platform::main_ext(b));
    let appimage = name(&linux, Bundle::AppImage);
    let arm = Target::parse("aarch64-unknown-linux-gnu").expect("static triple");
    let appimage_arm = name(&arm, Bundle::AppImage);
    let web_arm = format!(
        "{}.{}",
        platform::web_package_name(&v, &arm),
        arm.archive_ext()
    );
    let web = format!(
        "{}.{}",
        platform::web_package_name(&v, &linux),
        linux.archive_ext()
    );

    let mut s = String::new();
    s.push_str("\n\n---\n\n### ダウンロード / Downloads\n\n");
    s.push_str("| OS | ファイル / File | 備考 / Notes |\n|---|---|---|\n");
    // WiX cannot build an .msi for versions such as 2.1.0-rc.1 (NSIS only, see dist.rs).
    let msi = if version.msi_compatible() {
        format!(" MSI: {}", link(&name(&win, Bundle::Msi)))
    } else {
        String::new()
    };
    s.push_str(&format!(
        "| Windows 10/11 (x64) | {} | インストーラー（推奨） / Installer (recommended).{msi} |\n",
        link(&name(&win, Bundle::Nsis)),
    ));
    s.push_str(&format!(
        "| macOS 10.15+ (Apple Silicon / Intel) | {} | Universal |\n",
        link(&name(&mac, Bundle::Dmg)),
    ));
    s.push_str(&format!(
        "| Linux (x86_64) | {} | AppImage: `chmod +x {appimage} && ./{appimage}` |\n",
        link(&appimage),
    ));
    s.push_str(&format!(
        "| Debian / Ubuntu (x86_64) | {} | `sudo apt install ./<file>.deb` |\n",
        link(&name(&linux, Bundle::Deb)),
    ));
    s.push_str(&format!(
        "| Fedora / openSUSE (x86_64) | {} | `sudo dnf install ./<file>.rpm` |\n",
        link(&name(&linux, Bundle::Rpm)),
    ));
    s.push_str(&format!(
        "| Web (セルフホスト / self-hosting) | {} | `docker pull ghcr.io/{owner}/realamadeus-web:{v}` |\n",
        link(&web),
    ));
    // Linux arm64 (Raspberry Pi 4/5, Ampere, Apple Silicon VMs, ...). The Docker image is amd64
    // only, so the ARM self-host build is the archive.
    s.push_str(&format!(
        "| Linux (ARM64) | {} | AppImage: `chmod +x {appimage_arm} && ./{appimage_arm}` |\n",
        link(&appimage_arm),
    ));
    s.push_str(&format!(
        "| Debian / Ubuntu (ARM64) | {} | `sudo apt install ./<file>.deb` |\n",
        link(&name(&arm, Bundle::Deb)),
    ));
    s.push_str(&format!(
        "| Fedora / openSUSE (ARM64) | {} | `sudo dnf install ./<file>.rpm` |\n",
        link(&name(&arm, Bundle::Rpm)),
    ));
    s.push_str(&format!(
        "| Web ARM64 (セルフホスト / self-hosting) | {} | `tar xzf <file>.tar.gz` → `README-web.txt` |\n",
        link(&web_arm),
    ));
    s.push_str(&format!(
        "\n- ブラウザ版 / Browser version: {WEB_URL}\n\
         - チェックサム / Checksums: `SHA256SUMS.txt`\n\
         - Windows: SmartScreen が表示されたら「詳細情報」→「実行」/ If SmartScreen appears, choose \"More info\" → \"Run anyway\".\n\
         - macOS: 初回は Finder で右クリック →「開く」/ First launch: right-click the app in Finder → \"Open\".\n\
         \n当プロジェクトは Steins;Gate 0 の非営利の二次創作です / A non-commercial Steins;Gate 0 fan project.\n"
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        s.parse().unwrap()
    }

    const SAMPLE: &str = "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n## [2.0.0] - 2026-11-16\n\n### Added\n\n- Rust rewrite.\n- Desktop apps.\n\n## [V1.4] - 2025-01-01\n\nOld.\n\n[Unreleased]: https://github.com/o/r/compare/RealAmadeusV2.0...HEAD\n[2.0.0]: https://github.com/o/r/releases/tag/RealAmadeusV2.0\n";

    #[test]
    fn extracts_a_section_body() {
        assert_eq!(
            section_body(SAMPLE, &v("2.0.0")).unwrap(),
            "### Added\n\n- Rust rewrite.\n- Desktop apps."
        );
        assert_eq!(
            section_body(SAMPLE, &v("v2.0.0")).unwrap().lines().count(),
            4
        );
        // Link references at the end of the last section are not part of the notes.
        assert_eq!(section_body(SAMPLE, &v("1.4.0")), None);
        assert!(section_body(SAMPLE, &v("2.0.1")).is_none());
    }

    #[test]
    fn last_section_drops_link_refs() {
        let text = "## [2.0.0]\n- a\n\n[2.0.0]: https://x/y\n";
        assert_eq!(section_body(text, &v("2.0.0")).unwrap(), "- a");
    }

    #[test]
    fn inserts_a_template_after_empty_unreleased() {
        let (out, how) = insert_section(SAMPLE, &v("2.0.1"), "2026-12-01");
        assert_eq!(how, Ensured::Template);
        let unreleased = out.find("## [Unreleased]").unwrap();
        let new = out.find("## [2.0.1] - 2026-12-01").unwrap();
        let old = out.find("## [2.0.0] - 2026-11-16").unwrap();
        assert!(unreleased < new && new < old);
        assert!(section_body(&out, &v("2.0.1"))
            .unwrap()
            .contains(PLACEHOLDER));
        assert!(
            out.contains("[Unreleased]: https://github.com/o/r/compare/RealAmadeusV2.0.1...HEAD")
        );
        assert!(out.contains(
            "[2.0.1]: https://github.com/o/r/compare/RealAmadeusV2.0...RealAmadeusV2.0.1"
        ));
        // Idempotent.
        let (again, how) = insert_section(&out, &v("2.0.1"), "2026-12-02");
        assert_eq!(how, Ensured::Existing);
        assert_eq!(again, out);
    }

    #[test]
    fn moves_unreleased_notes_into_the_new_section() {
        let text = SAMPLE.replace(
            "## [Unreleased]\n",
            "## [Unreleased]\n\n### Fixed\n\n- Bug.\n",
        );
        let (out, how) = insert_section(&text, &v("2.0.1"), "2026-12-01");
        assert_eq!(how, Ensured::FromUnreleased);
        assert_eq!(
            section_body(&out, &v("2.0.1")).unwrap(),
            "### Fixed\n\n- Bug."
        );
        let lines: Vec<&str> = out.lines().collect();
        let secs = sections(&lines);
        let unreleased = secs
            .iter()
            .find(|s| s.heading == Heading::Unreleased)
            .unwrap();
        assert!(body(&lines, unreleased).is_empty());
    }

    #[test]
    fn works_without_unreleased_and_keeps_crlf() {
        let text = "# Changelog\r\n\r\n## [2.0.0] - 2026-11-16\r\n- a\r\n";
        let (out, how) = insert_section(text, &v("2.1.0-rc.1"), "2026-12-01");
        assert_eq!(how, Ensured::Template);
        assert!(out.contains("\r\n## [2.1.0-rc.1] - 2026-12-01\r\n"));
        assert!(out.find("2.1.0-rc.1").unwrap() < out.find("[2.0.0]").unwrap());
        assert!(!out.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn creates_a_file_from_the_header() {
        let (out, how) = insert_section(HEADER, &v("2.0.0"), "2026-11-16");
        assert_eq!(how, Ensured::Template);
        assert!(out.starts_with("# Changelog"));
        assert!(out.find("## [Unreleased]").unwrap() < out.find("## [2.0.0]").unwrap());
    }

    #[test]
    fn detects_whether_notes_are_ready() {
        let ready = |text: &str, ver: &str| {
            let (out, _) = insert_section(text, &v(ver), "2026-12-01");
            section_body(&out, &v(ver)).is_some_and(|b| has_real_notes(&b))
        };
        // Existing section with content.
        assert!(ready(SAMPLE, "2.0.0"));
        // New version, empty [Unreleased]: only the template would be written.
        assert!(!ready(SAMPLE, "2.0.1"));
        // New version, [Unreleased] has notes that move into it.
        let with_notes = SAMPLE.replace("## [Unreleased]\n", "## [Unreleased]\n\n- Fix.\n");
        assert!(ready(&with_notes, "2.0.1"));
        // An existing section that still holds the template text.
        let (templated, _) = insert_section(SAMPLE, &v("2.0.1"), "2026-12-01");
        assert!(!ready(&templated, "2.0.1"));
        // Link references and comments alone are not notes.
        assert!(!ready(
            "## [2.0.1]\n<!-- todo -->\n[2.0.1]: https://x/y\n",
            "2.0.1"
        ));
    }

    #[test]
    fn guide_lists_every_platform() {
        let guide = download_guide(&v("2.0.0"));
        for needle in [
            "RealAmadeus_V2.0_Windows_x64-setup.exe",
            "RealAmadeus_V2.0_Windows_x64.msi",
            "RealAmadeus_V2.0_macOS_universal.dmg",
            "chmod +x RealAmadeus_V2.0_Linux_amd64.AppImage && ./RealAmadeus_V2.0_Linux_amd64.AppImage",
            "RealAmadeus_V2.0_Linux_amd64.deb",
            "RealAmadeus_V2.0_Linux_x86_64.rpm",
            "RealAmadeus_V2.0_Web_Linux-x64.tar.gz",
            "RealAmadeus_V2.0_Linux_aarch64.AppImage",
            "RealAmadeus_V2.0_Linux_arm64.deb",
            "RealAmadeus_V2.0_Linux_aarch64.rpm",
            "RealAmadeus_V2.0_Web_Linux-arm64.tar.gz",
            "/releases/download/RealAmadeusV2.0/",
        ] {
            assert!(guide.contains(needle), "missing {needle}");
        }
        let rc = download_guide(&v("2.1.0-rc.1"));
        assert!(rc.contains("RealAmadeus_V2.1.0-rc.1_Windows_x64-setup.exe"));
        assert!(!rc.contains(".msi"), "no MSI is built for 2.1.0-rc.1");
    }

    #[test]
    fn the_real_changelog_has_the_current_version() {
        let current = version::current().unwrap();
        assert!(has_section(&path(), &current).unwrap());
    }
}
