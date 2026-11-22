//! `cargo xtask dist web|desktop|all` — builds release artifacts into `release/`.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, bail, Context, Result};

use crate::{
    archive,
    platform::{self, Bundle, Os, Target},
    util::{self, PRODUCT},
    version,
};

/// Placeholder that must be replaced by the real updater public key.
pub const PUBKEY_PLACEHOLDER: &str = "REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY";

/// README shipped inside the web package.
const README_WEB: &str = include_str!("../templates/README-web.txt");

/// README contents-list text of `amadeus-local-proxy` (Japanese). The binary has no options
/// besides `-h`/`--help` and `-V`/`--version`, so the settings are spelled out here (PLAT-009).
const LOCAL_PROXY_README_JA: &str = "\
任意: V1 の scripts/amadeus-local-proxy の Rust 版（プロバイダー 7 用。gcloud の
認証で Vertex AI に中継）。設定は環境変数だけ: LOCAL_PROXY_HOST（既定 0.0.0.0 =
すべてのインターフェース。この PC からだけ使うなら 127.0.0.1）、LOCAL_PROXY_PORT
（既定 5103）、VERTEX_PROJECT、VERTEX_LOCATION（既定 us-central1）。
-h/--help（使い方）と -V/--version 以外の引数は無視されます";

/// README contents-list text of `amadeus-local-proxy` (English).
const LOCAL_PROXY_README_EN: &str = "\
optional Rust port of V1's scripts/amadeus-local-proxy (provider 7: relays to
Vertex AI with your gcloud login). Environment variables only: LOCAL_PROXY_HOST
(default 0.0.0.0 = every interface; 127.0.0.1 keeps it on this machine),
LOCAL_PROXY_PORT (default 5103), VERTEX_PROJECT, VERTEX_LOCATION (default
us-central1). Arguments other than -h/--help (usage) and -V/--version are ignored";

pub struct WebOptions {
    pub base: String,
    pub target: Option<String>,
}

pub struct DesktopOptions {
    pub target: Option<String>,
    pub updater: bool,
    pub bundles: Option<String>,
    pub no_strip: bool,
}

// ---------------------------------------------------------------------------------------------
// Base path
// ---------------------------------------------------------------------------------------------

/// The URL prefix the web version is served under, in the two spellings that are needed.
#[derive(Debug, PartialEq, Eq)]
pub struct BasePath {
    /// Trunk `public_url`, always with leading and trailing slash (`/amadeus/`, `/`).
    pub public_url: String,
    /// `AMADEUS_BASE_PATH` of amadeus-server (`/amadeus`; `/` for the site root).
    pub server: String,
}

pub fn normalize_base(input: &str) -> Result<BasePath> {
    let raw = input.trim();
    if raw.contains("://") {
        bail!("--base must be a path such as /amadeus/ (amadeus-server serves the files itself), got `{raw}`");
    }
    if raw
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '?' | '#' | '\\' | '"' | '\''))
    {
        bail!("--base contains characters that are not allowed in a URL path: `{raw}`");
    }
    let trimmed = raw.trim_matches('/');
    if trimmed.split('/').any(|seg| seg == ".." || seg == ".") {
        bail!("--base must not contain `.` or `..` segments: `{raw}`");
    }
    Ok(if trimmed.is_empty() {
        BasePath {
            public_url: "/".into(),
            server: "/".into(),
        }
    } else {
        BasePath {
            public_url: format!("/{trimmed}/"),
            server: format!("/{trimmed}"),
        }
    })
}

/// Puts `AMADEUS_BASE_PATH=<base>` into the packaged `.env.example` when it differs from the
/// server default, so the extracted package works as-is.
fn env_example_for(template: &str, base: &BasePath) -> String {
    if base.server == "/amadeus" {
        return template.to_string();
    }
    let line = format!("AMADEUS_BASE_PATH={}", base.server);
    let mut replaced = false;
    let mut out: Vec<String> = template
        .lines()
        .map(|l| {
            let t = l.trim_start().trim_start_matches('#').trim_start();
            if !replaced && t.starts_with("AMADEUS_BASE_PATH=") {
                replaced = true;
                format!(
                    "{line}   # this package was built with --base {}",
                    base.public_url
                )
            } else {
                l.to_string()
            }
        })
        .collect();
    if !replaced {
        out.push(line);
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------------------------
// dist web
// ---------------------------------------------------------------------------------------------

pub fn web(opts: &WebOptions) -> Result<Vec<PathBuf>> {
    let parsed_version = version::current()?;
    let version = parsed_version.to_string();
    let label = parsed_version.label();
    let host = util::host_triple()?;
    let target = Target::parse(opts.target.as_deref().unwrap_or(&host))?;
    let base = normalize_base(&opts.base)?;
    let root = util::root();
    let target_dir = util::target_dir()?;
    let scratch = util::scratch_dir()?;
    if let Some(t) = &opts.target {
        ensure_rust_targets(&[t.as_str()])?;
    }

    // 1. Frontend (WASM) with Trunk.
    let web_dist = scratch.join("web-dist");
    util::remove_dir_if_exists(&web_dist)?;
    util::step(format!(
        "Building the web frontend with Trunk (public_url {})",
        base.public_url
    ));
    util::run(
        util::command("trunk")
            .current_dir(root.join("apps/web"))
            .args([
                "build",
                "--release",
                "--public-url",
                &base.public_url,
                "--dist",
            ])
            .arg(&web_dist),
    )?;
    let index = web_dist.join("index.html");
    if !index.is_file() {
        bail!("Trunk did not produce {}", index.display());
    }
    // amadeus-server serves the frontend only under the base it was built for.
    if !has_base_href(&util::read_to_string(&index)?, &base.public_url) {
        bail!(
            "{} has no <base href=\"{}\">: Trunk did not apply --public-url",
            index.display(),
            base.public_url
        );
    }

    // 2. Server binary.
    util::step(format!("Building amadeus-server ({})", target.triple));
    let mut cargo = util::cargo();
    cargo.args(["build", "--release", "-p", "amadeus-server"]);
    if let Some(t) = &opts.target {
        cargo.args(["--target", t]);
    }
    if util::in_ci() {
        cargo.arg("--locked");
    }
    util::run(&mut cargo)?;
    let bin_name = format!("amadeus-server{}", target.exe_suffix());
    let bin = match &opts.target {
        Some(t) => target_dir.join(t).join("release").join(&bin_name),
        None => target_dir.join("release").join(&bin_name),
    };
    if !bin.is_file() {
        bail!("expected the server binary at {}", bin.display());
    }
    if target.triple == host {
        smoke_test_server(&bin, &version)?;
    }
    // Optional companion binary of the amadeus-server package (Rust port of V1's
    // scripts/amadeus-local-proxy); shipped when the package builds it.
    let proxy_name = format!("amadeus-local-proxy{}", target.exe_suffix());
    let proxy = bin.with_file_name(&proxy_name);

    // 3. Stage the package directory.
    let name = platform::web_package_name(&version, &target);
    let stage_parent = scratch.join("stage");
    let stage = stage_parent.join(&name);
    util::remove_dir_if_exists(&stage)?;
    util::step(format!("Staging {name}"));
    util::copy_file(&bin, &stage.join(&bin_name))?;
    let has_proxy = proxy.is_file();
    if has_proxy {
        util::copy_file(&proxy, &stage.join(&proxy_name))?;
    }
    util::copy_dir_all(&web_dist, &stage.join("dist"), &|n| {
        n.to_string_lossy().starts_with(".trunk") || n == ".stage"
    })?;

    let deploy = root.join("deploy");
    let env_template = util::read_to_string(&deploy.join(".env.example"))?;
    util::write(
        &stage.join(".env.example"),
        env_example_for(&env_template, &base),
    )?;
    util::copy_dir_all(&deploy, &stage.join("deploy"), &|n| {
        n == ".env.example" || n.to_string_lossy().starts_with('.')
    })?;
    util::copy_file(&root.join("LICENSE"), &stage.join("LICENSE"))?;

    let slug = crate::changelog::repository_slug();
    let url_example = format!("http://127.0.0.1:9502{}", base.public_url);
    let readme = util::render(
        README_WEB,
        &[
            ("product", PRODUCT),
            ("label", &label),
            ("version", &version),
            ("name", &name),
            ("bin", &bin_name),
            ("base", &base.server),
            ("public_url", &base.public_url),
            ("url", &url_example),
            ("platform", target.platform_label()),
            ("triple", &target.triple),
            ("repo", &slug),
            (
                "extra_ja",
                &extra_line(has_proxy, &proxy_name, LOCAL_PROXY_README_JA),
            ),
            (
                "extra_en",
                &extra_line(has_proxy, &proxy_name, LOCAL_PROXY_README_EN),
            ),
        ],
    );
    let readme = if target.os == Os::Windows {
        readme.replace('\n', "\r\n")
    } else {
        readme
    };
    util::write(&stage.join("README-web.txt"), readme)?;

    // 4. Archive.
    let release = util::release_dir();
    let ext = target.archive_ext();
    let out = release.join(format!("{name}.{ext}"));
    util::step(format!("Packing {}", util::rel(&out)));
    archive::create(&stage_parent, &name, &out, archive::Format::from_ext(ext)?)?;

    let outputs = vec![out];
    finish(
        &release,
        &outputs,
        &format!("Web package ({})", target.triple),
    )?;
    Ok(outputs)
}

/// True when `html` has a `<base href>` equal to `public_url`, however it is quoted: Trunk's
/// release minifier writes `<base href=/amadeus/>`, debug builds `<base href="/amadeus/" />`
/// (after the commented-out template tag of `index.html`, which never matches).
fn has_base_href(html: &str, public_url: &str) -> bool {
    let lower = html.to_ascii_lowercase(); // same byte offsets as `html`
    let mut from = 0;
    while let Some(pos) = lower[from..].find("<base") {
        let start = from + pos;
        let end = lower[start..].find('>').map_or(lower.len(), |e| start + e);
        if let Some(h) = lower[start..end].find("href=") {
            let rest = &html[start + h + "href=".len()..end];
            let value = match rest.chars().next() {
                Some(q @ ('"' | '\'')) => rest[1..].split(q).next().unwrap_or_default(),
                _ => rest.split_ascii_whitespace().next().unwrap_or_default(),
            };
            if value == public_url {
                return true;
            }
        }
        from = end;
    }
    false
}

/// Runs `amadeus-server --version` and checks that the binary carries the workspace version
/// (catches a stale binary from another target directory). Only possible for the host target.
fn smoke_test_server(bin: &Path, version: &str) -> Result<()> {
    let scratch = util::scratch_dir()?;
    fs::create_dir_all(&scratch)?;
    // From an empty directory, so no stray `.env` is read; AMADEUS_ENV_FILE is cleared for the
    // same reason.
    let out = util::capture(
        util::command(bin)
            .arg("--version")
            .current_dir(&scratch)
            .env_remove("AMADEUS_ENV_FILE"),
    )?;
    let reported = out.trim();
    if !version_line_matches(reported, version) {
        bail!(
            "{} reports `{reported}`, expected version {version}",
            bin.display()
        );
    }
    util::info(format!("smoke test: {reported}"));
    Ok(())
}

/// True when clap's `--version` line (`amadeus-server 2.0.0`) names exactly `version`.
fn version_line_matches(line: &str, version: &str) -> bool {
    line.split_whitespace().nth(1) == Some(version)
}

/// An entry for the README contents list, or nothing when the file is not shipped. Further
/// lines of `text` are indented to the column where its first line starts.
fn extra_line(present: bool, file: &str, text: &str) -> String {
    if !present {
        return String::new();
    }
    let mut lines = text.lines();
    let mut out = format!("  {file:<12} {}\n", lines.next().unwrap_or_default());
    let indent = " ".repeat(2 + file.chars().count().max(12) + 1);
    for line in lines {
        out.push_str(&format!("{indent}{line}\n"));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// dist desktop
// ---------------------------------------------------------------------------------------------

fn rustup_targets() -> Option<BTreeSet<String>> {
    util::probe(util::command("rustup").args(["target", "list", "--installed"]))
        .map(|s| s.lines().map(|l| l.trim().to_string()).collect())
}

fn ensure_rust_targets(needed: &[&str]) -> Result<()> {
    let Some(installed) = rustup_targets() else {
        util::warn("rustup not found; cannot verify that the Rust targets are installed");
        return Ok(());
    };
    let missing: Vec<&str> = needed
        .iter()
        .copied()
        .filter(|t| !installed.contains(*t))
        .collect();
    if !missing.is_empty() {
        util::step(format!("Installing Rust targets: {}", missing.join(", ")));
        util::run(
            util::command("rustup")
                .args(["target", "add"])
                .args(&missing),
        )?;
    }
    Ok(())
}

/// The Tauri CLI: `cargo tauri` by default, or the command in `TAURI_CLI` (e.g. `tauri` from
/// `npm i -g @tauri-apps/cli`; on Windows npm installs it as `tauri.cmd`).
fn tauri_cli() -> Command {
    match util::env_non_empty("TAURI_CLI") {
        Some(cli) => {
            let mut parts = cli.split_whitespace();
            let mut cmd = util::command(parts.next().unwrap_or("tauri"));
            cmd.args(parts);
            cmd
        }
        None => {
            let mut cmd = util::cargo();
            cmd.arg("tauri");
            cmd
        }
    }
}

fn ensure_tauri_cli() -> Result<()> {
    let mut cmd = tauri_cli();
    let shown = util::describe(&cmd);
    match util::probe(cmd.arg("--version")) {
        Some(v) => util::info(format!("Tauri CLI: {}", v.trim())),
        None => bail!(
            "the Tauri CLI (`{shown}`) is not available. Install it with \
             `cargo install tauri-cli --version \"^2.12\" --locked`, or install \
             `npm i -g @tauri-apps/cli@2` and set TAURI_CLI=tauri (tauri.cmd on Windows)."
        ),
    }
    Ok(())
}

/// Writes the updater override config to `target/xtask/`, applying `TAURI_UPDATER_PUBKEY`
/// if set, and refuses to build with the placeholder key.
fn updater_config() -> Result<PathBuf> {
    let src = util::root().join("apps/desktop/tauri.updater.conf.json");
    let text = util::read_to_string(&src)?;
    let mut json: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("{} is not valid JSON", util::rel(&src)))?;
    if let Some(key) = util::env_non_empty("TAURI_UPDATER_PUBKEY") {
        let updater = json
            .pointer_mut("/plugins/updater")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| anyhow!("{} has no plugins.updater object", util::rel(&src)))?;
        updater.insert(
            "pubkey".into(),
            serde_json::Value::String(key.trim().to_string()),
        );
    }
    let pubkey = json
        .pointer("/plugins/updater/pubkey")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if pubkey.is_empty() || pubkey.contains(PUBKEY_PLACEHOLDER) {
        bail!(
            "the updater public key is not configured. Paste the public key from \
             `cargo tauri signer generate` into plugins.updater.pubkey of {} \
             (or set TAURI_UPDATER_PUBKEY). See docs/RELEASE.md.",
            util::rel(&src)
        );
    }
    let out = util::scratch_dir()?.join("tauri.updater.conf.json");
    util::write(&out, serde_json::to_string_pretty(&json)? + "\n")?;
    Ok(out)
}

/// `--bundles` for `tauri build`: the user's choice, or NSIS only for Windows when the version
/// cannot be expressed as an MSI version (e.g. `2.1.0-rc.1`), so pre-releases still build.
fn effective_bundles(
    requested: Option<&str>,
    target: &Target,
    version: &version::Version,
) -> Option<String> {
    match requested {
        Some(list) => Some(list.to_string()),
        None if target.os == Os::Windows && !version.msi_compatible() => Some("nsis".to_string()),
        None => None,
    }
}

/// Why `bundle.licenseFile` of a Tauri config would break the installers, if it would.
///
/// tauri-bundler 2.10.x (pinned by Tauri CLI 2.12.x) wraps a plain-text license in an RTF
/// document for the MSI without escaping it: a character outside Windows-1252 fails WiX with
/// LGHT0311 (database code page 1252) and `\`, `{`, `}` are read as RTF syntax; a `.rtf` file
/// is wrapped a second time. The DMG shows non-ASCII text garbled. Until the pinned CLI has
/// tauri-apps/tauri#16493, an installer license must be plain ASCII text (the bilingual
/// LICENSE is not).
fn installer_license_problem(conf_path: &Path) -> Result<Option<String>> {
    let conf: serde_json::Value = serde_json::from_str(&util::read_to_string(conf_path)?)
        .with_context(|| format!("{} is not valid JSON", util::rel(conf_path)))?;
    let Some(file) = conf.pointer("/bundle/licenseFile").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    let path = conf_path.parent().unwrap_or(Path::new(".")).join(file);
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rtf"))
    {
        return Ok(Some(format!(
            "bundle.licenseFile `{file}` is an RTF file, which the MSI bundler wraps in a second \
             RTF document"
        )));
    }
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read bundle.licenseFile {}", path.display()))?;
    Ok(bytes
        .iter()
        .position(|b| !b.is_ascii() || matches!(b, b'\\' | b'{' | b'}'))
        .map(|pos| {
            let line = bytes[..pos].iter().filter(|&&b| b == b'\n').count() + 1;
            format!(
                "bundle.licenseFile `{file}` has a non-ASCII character or one of `\\ {{ }}` \
                 (line {line}): the MSI build fails with WiX LGHT0311 and the DMG shows it garbled"
            )
        }))
}

pub fn desktop(opts: &DesktopOptions) -> Result<Vec<PathBuf>> {
    let parsed_version = version::current()?;
    let version = parsed_version.to_string();
    let host = Target::parse(&util::host_triple()?)?;
    let target = match &opts.target {
        Some(t) => Target::parse(t)?,
        None => host.clone(),
    };
    if target.os != host.os {
        bail!(
            "desktop bundles for {} must be built on {} (Tauri cannot cross-bundle reliably). \
             Use the release workflow, which builds every platform on its own runner.",
            target.triple,
            target.platform_label()
        );
    }
    if let Some(t) = &opts.target {
        if t == "universal-apple-darwin" {
            ensure_rust_targets(&["aarch64-apple-darwin", "x86_64-apple-darwin"])?;
        } else {
            ensure_rust_targets(&[t.as_str()])?;
        }
    }
    ensure_tauri_cli()?;

    let root = util::root();
    let target_dir = util::target_dir()?;
    let bundle_dir = match &opts.target {
        Some(t) => target_dir.join(t).join("release").join("bundle"),
        None => target_dir.join("release").join("bundle"),
    };
    // Stale bundles of an older version must never be picked up.
    util::remove_dir_if_exists(&bundle_dir)?;

    let mut cmd = tauri_cli();
    cmd.current_dir(root.join("apps/desktop")).arg("build");
    if let Some(t) = &opts.target {
        cmd.args(["--target", t]);
    }
    let bundles = effective_bundles(opts.bundles.as_deref(), &target, &parsed_version);
    if opts.bundles.is_none() && bundles.is_some() {
        util::warn(format!(
            "WiX cannot build an .msi for version {version} (a pre-release must be a single \
             number such as 2.1.0-1); building the NSIS installer only"
        ));
    }
    // WiX would only fail at the very end of the build.
    if let Some(problem) = installer_license_problem(&root.join("apps/desktop/tauri.conf.json"))? {
        let builds = |format: &str| {
            bundles.as_deref().is_none_or(|list| {
                list.split(',')
                    .any(|b| b.trim().eq_ignore_ascii_case(format))
            })
        };
        if target.os == Os::Windows && builds("msi") {
            bail!("{problem}. Use a plain-ASCII text file or remove the key (docs/RELEASE.md §5).");
        }
        if target.os == Os::Mac && builds("dmg") {
            util::warn(format!("{problem}."));
        }
    }
    if let Some(b) = &bundles {
        cmd.args(["--bundles", b]);
    }
    if opts.updater {
        if util::env_non_empty("TAURI_SIGNING_PRIVATE_KEY").is_none() {
            bail!(
                "--updater needs TAURI_SIGNING_PRIVATE_KEY (the key from `cargo tauri signer generate`, \
                 or a path to it) and, if it has one, TAURI_SIGNING_PRIVATE_KEY_PASSWORD"
            );
        }
        let conf = updater_config()?;
        cmd.args(["--features", "updater", "--config"]).arg(conf);
    }
    if target.os == Os::Linux {
        // linuxdeploy (used for the AppImage) is itself an AppImage; without FUSE it can still
        // run by extracting itself.
        if !Path::new("/dev/fuse").exists()
            && std::env::var_os("APPIMAGE_EXTRACT_AND_RUN").is_none()
        {
            util::info("/dev/fuse is missing: setting APPIMAGE_EXTRACT_AND_RUN=1 for linuxdeploy");
            cmd.env("APPIMAGE_EXTRACT_AND_RUN", "1");
        }
        if opts.no_strip {
            cmd.env("NO_STRIP", "true");
        }
    }

    util::step(format!(
        "Building the desktop app with Tauri ({}{})",
        target.triple,
        if opts.updater {
            ", updater enabled"
        } else {
            ""
        }
    ));
    util::run(&mut cmd)?;

    let release = util::release_dir();
    let outputs = collect_bundles(&bundle_dir, &target, &version, &release)?;
    if outputs.is_empty() {
        bail!(
            "Tauri finished but no bundles were found in {}",
            bundle_dir.display()
        );
    }
    finish(
        &release,
        &outputs,
        &format!("Desktop bundles ({})", target.triple),
    )?;
    Ok(outputs)
}

fn sorted_entries(dir: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .collect::<std::io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

/// Copies Tauri's bundles into `release/` under the normalized names.
fn collect_bundles(
    bundle_dir: &Path,
    target: &Target,
    version: &str,
    release: &Path,
) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(release)?;
    let mut outputs = Vec::new();
    let mut used = BTreeSet::new();
    for bundle in Bundle::ALL {
        if bundle.os() != target.os {
            continue;
        }
        let dir = bundle_dir.join(bundle.dir());
        if !dir.is_dir() {
            continue;
        }
        if bundle == Bundle::App {
            tar_app_bundles(&dir)?;
        }
        for entry in sorted_entries(&dir)? {
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().into_owned();
            // Tauri names every bundle after productName; anything else (tool downloads,
            // logs) is not ours.
            if !file_name.starts_with(PRODUCT) {
                continue;
            }
            let Some(ext) = bundle.extensions().iter().find(|e| file_name.ends_with(*e)) else {
                continue;
            };
            let mut name = platform::bundle_asset_name(version, target, bundle, ext);
            if !used.insert(name.clone()) {
                // Several WiX languages produce several .msi files: keep them apart.
                let lang = file_name
                    .trim_end_matches(ext)
                    .rsplit('_')
                    .next()
                    .unwrap_or("alt")
                    .to_string();
                name =
                    platform::bundle_asset_name(version, target, bundle, &format!("_{lang}{ext}"));
                used.insert(name.clone());
            }
            let out = release.join(&name);
            util::copy_file(&entry.path(), &out)?;
            util::info(format!("{file_name}  ->  {}", util::rel(&out)));
            outputs.push(out);
        }
    }
    Ok(outputs)
}

/// macOS: ship the `.app` as `.app.tar.gz` too (like tauri-action) when Tauri did not
/// already create one for the updater.
fn tar_app_bundles(dir: &Path) -> Result<()> {
    for entry in sorted_entries(dir)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".app") || !entry.file_type()?.is_dir() {
            continue;
        }
        let tarball = dir.join(format!("{name}.tar.gz"));
        if !tarball.exists() {
            archive::create(dir, &name, &tarball, archive::Format::TarGz)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------------------------

/// Writes `.sha256` side files, refreshes `SHA256SUMS.txt` and prints a summary.
fn finish(release: &Path, outputs: &[PathBuf], title: &str) -> Result<()> {
    util::step(format!(
        "{title}: {} file(s) in {}",
        outputs.len(),
        util::rel(release)
    ));
    let mut summary = format!("### {title}\n\n| File | Size | SHA-256 |\n|---|---|---|\n");
    for out in outputs {
        let name = out.file_name().unwrap_or_default().to_string_lossy();
        let size = fs::metadata(out).map(|m| m.len()).unwrap_or(0);
        let hash = if util::is_distributable(&name) {
            util::write_sha256_file(out)?
        } else {
            util::sha256_file(out)?
        };
        // stdout: `sha256sum`-compatible lines.
        println!("{hash}  {name}");
        util::info(format!("{name} ({})", util::human_size(size)));
        summary.push_str(&format!(
            "| `{name}` | {} | `{hash}` |\n",
            util::human_size(size)
        ));
    }
    let sums = util::update_sha256sums(release)?;
    util::info(format!("checksums: {}", util::rel(&sums)));
    util::job_summary(&(summary + "\n"));
    Ok(())
}

/// `cargo xtask clean-release`
pub fn clean_release() -> Result<()> {
    let release = util::release_dir();
    util::remove_dir_if_exists(&release)?;
    util::info(format!("removed {}", util::rel(&release)));
    let scratch = util::scratch_dir()?;
    util::remove_dir_if_exists(&scratch)?;
    util::info(format!("removed {}", util::rel(&scratch)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_paths() {
        let b = normalize_base("/amadeus/").unwrap();
        assert_eq!(
            (b.public_url.as_str(), b.server.as_str()),
            ("/amadeus/", "/amadeus")
        );
        let b = normalize_base("amadeus-v2").unwrap();
        assert_eq!(
            (b.public_url.as_str(), b.server.as_str()),
            ("/amadeus-v2/", "/amadeus-v2")
        );
        let b = normalize_base("/").unwrap();
        assert_eq!((b.public_url.as_str(), b.server.as_str()), ("/", "/"));
        let b = normalize_base("/apps/amadeus").unwrap();
        assert_eq!(b.public_url, "/apps/amadeus/");
        assert!(normalize_base("https://cdn.example.com/").is_err());
        assert!(normalize_base("/a b/").is_err());
        assert!(normalize_base("/../x/").is_err());
    }

    #[test]
    fn base_href_detection() {
        let minified =
            "<!doctype html><html><meta charset=utf-8><base href=/amadeus/><title>A</title>";
        assert!(has_base_href(minified, "/amadeus/"));
        assert!(!has_base_href(minified, "/amadeus-v2/"));
        let debug = "<!-- Trunk replaces this with <base href=\"{public_url}\">; ... -->\n    \
                     <base href=\"/amadeus/\" />\n";
        assert!(has_base_href(debug, "/amadeus/"));
        assert!(has_base_href("<BASE HREF='/x/'>", "/x/"));
        assert!(has_base_href("<base href=/>", "/"));
        assert!(has_base_href("<base href=\"/\" />", "/"));
        assert!(!has_base_href(
            "<base data-trunk-public-url />",
            "/amadeus/"
        ));
        assert!(!has_base_href("<html></html>", "/"));
    }

    #[test]
    fn server_version_lines() {
        assert!(version_line_matches("amadeus-server 2.0.0", "2.0.0"));
        assert!(version_line_matches(
            "amadeus-server 2.1.0-rc.1\n",
            "2.1.0-rc.1"
        ));
        assert!(!version_line_matches("amadeus-server 2.0.0", "2.0.1"));
        assert!(!version_line_matches("amadeus-server 2.0.10", "2.0.1"));
        assert!(!version_line_matches("", "2.0.0"));
    }

    #[test]
    fn readme_template_is_fully_rendered() {
        let keys = [
            "product",
            "label",
            "version",
            "name",
            "bin",
            "base",
            "public_url",
            "url",
            "platform",
            "triple",
            "repo",
            "extra_ja",
            "extra_en",
        ];
        let vars: Vec<(&str, &str)> = keys.iter().map(|k| (*k, "x")).collect();
        let out = util::render(README_WEB, &vars);
        assert!(
            !out.contains("{{"),
            "unrendered placeholder in README-web.txt"
        );
        assert_eq!(extra_line(false, "a", "b"), "");
        assert_eq!(
            extra_line(true, "amadeus-local-proxy", "t"),
            "  amadeus-local-proxy t\n"
        );
        assert_eq!(
            extra_line(true, "amadeus-local-proxy", "a\nb"),
            "  amadeus-local-proxy a\n                      b\n"
        );
        assert_eq!(
            extra_line(true, "x", "a\nb"),
            "  x            a\n               b\n"
        );
    }

    /// PLAT-009: the README names the proxy's settings instead of pointing at `--help`.
    #[test]
    fn readme_names_the_local_proxy_settings() {
        for text in [LOCAL_PROXY_README_JA, LOCAL_PROXY_README_EN] {
            for setting in [
                "LOCAL_PROXY_HOST",
                "0.0.0.0",
                "127.0.0.1",
                "LOCAL_PROXY_PORT",
                "5103",
                "VERTEX_PROJECT",
                "VERTEX_LOCATION",
                "us-central1",
                "-h/--help",
                "-V/--version",
            ] {
                assert!(text.contains(setting), "{setting} missing in {text}");
            }
            assert!(!text.contains("--help 参照") && !text.contains("see --help"));
        }
        let vars: Vec<(&str, String)> = [
            ("bin", "amadeus-server".to_owned()),
            (
                "extra_ja",
                extra_line(true, "amadeus-local-proxy", LOCAL_PROXY_README_JA),
            ),
            (
                "extra_en",
                extra_line(true, "amadeus-local-proxy", LOCAL_PROXY_README_EN),
            ),
        ]
        .into_iter()
        .collect();
        let refs: Vec<(&str, &str)> = vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let readme = util::render(README_WEB, &refs);
        assert!(readme.contains("\n  amadeus-local-proxy 任意: V1 の scripts/amadeus-local-proxy"));
        assert!(readme.contains("\n                      LOCAL_PROXY_PORT (default 5103)"));
        assert!(readme.contains("AMADEUS_FREEAPI_MAX"));
        for line in readme.lines() {
            assert!(line.chars().count() <= 110, "README line too long: {line}");
        }
    }

    #[test]
    fn env_example_gets_the_base_path() {
        let template = "AI_STUDIO_API_KEY=\n# AMADEUS_BASE_PATH=/amadeus\n# RUST_LOG=info\n";
        let default = normalize_base("/amadeus/").unwrap();
        assert_eq!(env_example_for(template, &default), template);
        let custom = normalize_base("/amadeus-v2/").unwrap();
        let out = env_example_for(template, &custom);
        assert!(out.contains("\nAMADEUS_BASE_PATH=/amadeus-v2   #"));
        assert!(!out.contains("# AMADEUS_BASE_PATH=/amadeus\n"));
    }

    #[test]
    fn collects_and_renames_bundles() {
        let tmp = std::env::temp_dir().join(format!("xtask-collect-{}", std::process::id()));
        let bundle = tmp.join("bundle");
        let release = tmp.join("release");
        for (dir, file) in [
            ("deb", "RealAmadeus_2.0.0_amd64.deb"),
            ("rpm", "RealAmadeus-2.0.0-1.x86_64.rpm"),
            ("appimage", "RealAmadeus_2.0.0_amd64.AppImage"),
            ("appimage", "RealAmadeus_2.0.0_amd64.AppImage.sig"),
            ("appimage", "linuxdeploy-x86_64.AppImage.log"),
            ("appimage", "linuxdeploy-x86_64.AppImage"),
        ] {
            fs::create_dir_all(bundle.join(dir)).unwrap();
            fs::write(bundle.join(dir).join(file), file).unwrap();
        }
        fs::create_dir_all(bundle.join("deb/RealAmadeus_2.0.0_amd64")).unwrap();
        let target = Target::parse("x86_64-unknown-linux-gnu").unwrap();
        let outputs = collect_bundles(&bundle, &target, "2.0.0", &release).unwrap();
        let names: Vec<String> = outputs
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "RealAmadeus_V2.0_Linux_amd64.AppImage",
                "RealAmadeus_V2.0_Linux_amd64.AppImage.sig",
                "RealAmadeus_V2.0_Linux_amd64.deb",
                "RealAmadeus_V2.0_Linux_x86_64.rpm",
            ]
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn prereleases_skip_the_msi_on_windows() {
        let win = Target::parse("x86_64-pc-windows-msvc").unwrap();
        let linux = Target::parse("x86_64-unknown-linux-gnu").unwrap();
        let rc: version::Version = "2.1.0-rc.1".parse().unwrap();
        let stable: version::Version = "2.1.0".parse().unwrap();
        assert_eq!(effective_bundles(None, &win, &rc).as_deref(), Some("nsis"));
        assert_eq!(effective_bundles(None, &win, &stable), None);
        assert_eq!(effective_bundles(None, &linux, &rc), None);
        assert_eq!(
            effective_bundles(Some("msi"), &win, &rc).as_deref(),
            Some("msi")
        );
    }

    #[test]
    fn msi_languages_do_not_collide() {
        let tmp = std::env::temp_dir().join(format!("xtask-msi-{}", std::process::id()));
        let bundle = tmp.join("bundle");
        fs::create_dir_all(bundle.join("msi")).unwrap();
        for f in [
            "RealAmadeus_2.0.0_x64_en-US.msi",
            "RealAmadeus_2.0.0_x64_ja-JP.msi",
        ] {
            fs::write(bundle.join("msi").join(f), f).unwrap();
        }
        let target = Target::parse("x86_64-pc-windows-msvc").unwrap();
        let outputs = collect_bundles(&bundle, &target, "2.0.0", &tmp.join("release")).unwrap();
        let names: Vec<String> = outputs
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "RealAmadeus_V2.0_Windows_x64.msi",
                "RealAmadeus_V2.0_Windows_x64_ja-JP.msi"
            ]
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn the_real_desktop_config_can_build_an_msi() {
        let conf = util::root().join("apps/desktop/tauri.conf.json");
        assert_eq!(installer_license_problem(&conf).unwrap(), None);
    }

    #[test]
    fn installer_license_must_be_plain_ascii() {
        let tmp = std::env::temp_dir().join(format!("xtask-license-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let conf = tmp.join("tauri.conf.json");
        let check = |license: Option<&str>, text: &str| {
            let bundle = match license {
                Some(file) => {
                    fs::write(tmp.join(file), text).unwrap();
                    serde_json::json!({ "active": true, "licenseFile": file })
                }
                None => serde_json::json!({ "active": true }),
            };
            fs::write(&conf, serde_json::json!({ "bundle": bundle }).to_string()).unwrap();
            installer_license_problem(&conf).unwrap()
        };
        assert_eq!(check(None, ""), None);
        assert_eq!(
            check(Some("LICENSE.txt"), "CC BY-NC 4.0\n- Share\r\n"),
            None
        );
        let japanese = check(Some("LICENSE.txt"), "CC BY-NC 4.0\n---\n本プロジェクト\n").unwrap();
        assert!(japanese.contains("line 3"), "{japanese}");
        assert!(check(Some("LICENSE.txt"), "see {braces}").is_some());
        assert!(check(Some("LICENSE.txt"), r"C:\path").is_some());
        assert!(check(Some("license.RTF"), r"{\rtf1 ok}").is_some());
        let _ = fs::remove_dir_all(&tmp);
    }
}
