//! Build & release automation for RealAmadeus V2. Run `cargo xtask help`.
//!
//! Local builds and the GitHub release workflow go through the same commands, so artifacts
//! are named identically everywhere (`RealAmadeus_V<version>_<Platform>_<arch>...`).

mod archive;
mod changelog;
mod check;
mod classes;
mod dist;
mod platform;
mod util;
mod version;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cargo xtask",
    bin_name = "cargo xtask",
    version,
    about = "Build and release automation for RealAmadeus V2 (Rust)",
    after_help = "Typical release:\n  \
        cargo xtask bump 2.0.1 --commit --tag   # version + CHANGELOG + commit + tag\n  \
        git push --follow-tags                  # GitHub Actions builds and publishes everything\n\n\
        Local artifacts:\n  \
        cargo xtask dist web                    # release/RealAmadeus_V<ver>_Web_<os>-<arch>.tar.gz\n  \
        cargo xtask dist desktop                # installers for the current OS\n\n\
        See docs/RELEASE.md for details."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the release version ([workspace.package] version in Cargo.toml)
    Version(VersionArgs),
    /// Set a new version in Cargo.toml / Cargo.lock and add its CHANGELOG.md section (never pushes)
    Bump(BumpArgs),
    /// Print the CHANGELOG.md section of a version (the GitHub release body)
    ReleaseNotes(ReleaseNotesArgs),
    /// Build release artifacts into release/
    Dist(DistArgs),
    /// Run the CI checks locally: fmt, clippy (native + wasm32), tests, Trunk builds, Tailwind class guard
    Check(CheckArgs),
    /// Check that every Tailwind arbitrary-value class in the UI sources is in a Trunk build's CSS
    CheckClasses(CheckClassesArgs),
    /// Delete release/ and the xtask scratch directory (target/xtask)
    CleanRelease,
    /// Shortcut for `dist web`
    #[command(hide = true)]
    Web(WebArgs),
    /// Shortcut for `dist desktop`
    #[command(hide = true)]
    Desktop(DesktopArgs),
}

#[derive(Args)]
#[group(multiple = false)]
struct VersionArgs {
    /// Print the Git tag instead (RealAmadeusV2.0)
    #[arg(long)]
    tag: bool,
    /// Print the product label instead (V2.0)
    #[arg(long)]
    label: bool,
    /// Print version, tag, label and asset prefix as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct BumpArgs {
    /// New version (SemVer), e.g. 2.0.1 or 2.1.0-rc.1
    version: String,
    /// Commit Cargo.toml, Cargo.lock and CHANGELOG.md
    #[arg(long)]
    commit: bool,
    /// Create the annotated tag `RealAmadeus<label>` (e.g. `RealAmadeusV2.0`) (push it yourself with `git push --follow-tags`)
    #[arg(long)]
    tag: bool,
    /// Release date for the CHANGELOG heading (default: today)
    #[arg(long, value_name = "YYYY-MM-DD")]
    date: Option<String>,
    /// Allow setting a lower version than the current one
    #[arg(long)]
    allow_downgrade: bool,
}

#[derive(Args)]
struct ReleaseNotesArgs {
    /// Version to print (default: the current workspace version); `v2.0.1` / `V2.0.1R` work too
    version: Option<String>,
    /// Append the download table (which file to pick per OS) used on GitHub releases
    #[arg(long)]
    download_guide: bool,
}

#[derive(Args)]
struct DistArgs {
    #[command(subcommand)]
    what: DistCommand,
}

#[derive(Subcommand)]
enum DistCommand {
    /// Web version: Trunk frontend + amadeus-server, packed as .tar.gz (.zip for Windows)
    Web(WebArgs),
    /// Desktop installers for the current OS via `cargo tauri build`
    Desktop(DesktopArgs),
    /// `dist web` followed by `dist desktop`
    All(AllArgs),
}

#[derive(Args)]
struct WebArgs {
    /// URL prefix the web version is served under (Trunk public_url / AMADEUS_BASE_PATH)
    #[arg(long, default_value = "/amadeus/", value_name = "PATH")]
    base: String,
    /// Target triple of the server binary (default: host)
    #[arg(long, value_name = "TRIPLE")]
    target: Option<String>,
}

#[derive(Args)]
struct DesktopArgs {
    /// Target triple, e.g. universal-apple-darwin (default: host)
    #[arg(long, value_name = "TRIPLE")]
    target: Option<String>,
    /// Build signed updater artifacts (needs TAURI_SIGNING_PRIVATE_KEY and the updater pubkey)
    #[arg(long)]
    updater: bool,
    /// Bundle formats passed to `tauri build --bundles`, e.g. deb,rpm,appimage
    #[arg(long, value_name = "LIST")]
    bundles: Option<String>,
    /// Linux: set NO_STRIP=true (works around linuxdeploy's old `strip` on new distros)
    #[arg(long)]
    no_strip: bool,
}

#[derive(Args)]
struct AllArgs {
    /// URL prefix for the web package
    #[arg(long, default_value = "/amadeus/", value_name = "PATH")]
    base: String,
    /// Target triple for the desktop build, e.g. universal-apple-darwin (default: host)
    #[arg(long, value_name = "TRIPLE")]
    desktop_target: Option<String>,
    /// Build signed updater artifacts for the desktop app
    #[arg(long)]
    updater: bool,
}

#[derive(Args)]
struct CheckArgs {
    /// Skip clippy for wasm32-unknown-unknown
    #[arg(long)]
    no_wasm: bool,
    /// Skip `cargo test`
    #[arg(long)]
    no_test: bool,
    /// Skip the Trunk build of the web frontend
    #[arg(long)]
    no_trunk: bool,
    /// Skip the desktop frontend build and the desktop clippy run
    #[arg(long)]
    no_desktop: bool,
    /// Use debug Trunk builds (faster, no wasm-opt)
    #[arg(long)]
    quick: bool,
}

#[derive(Args)]
struct CheckClassesArgs {
    /// Trunk output directory to check [default: apps/web/dist, where `trunk build` writes]
    #[arg(long, value_name = "DIR")]
    dist: Option<PathBuf>,
    /// Also print every class found and whether the CSS has it
    #[arg(long)]
    list: bool,
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Version(a) => version::print(if a.tag {
            version::VersionFormat::Tag
        } else if a.label {
            version::VersionFormat::Label
        } else if a.json {
            version::VersionFormat::Json
        } else {
            version::VersionFormat::Plain
        }),
        Command::Bump(a) => version::bump(version::BumpOptions {
            version: a.version,
            commit: a.commit,
            tag: a.tag,
            date: a.date,
            allow_downgrade: a.allow_downgrade,
        }),
        Command::ReleaseNotes(a) => {
            let v: version::Version = match a.version {
                Some(s) => s.parse()?,
                None => version::current()?,
            };
            let mut notes = changelog::release_notes(&v)?;
            if a.download_guide {
                notes.push_str(&changelog::download_guide(&v));
            }
            println!("{}", notes.trim_end());
            Ok(())
        }
        Command::Web(a) => run(Cli {
            command: Command::Dist(DistArgs {
                what: DistCommand::Web(a),
            }),
        }),
        Command::Desktop(a) => run(Cli {
            command: Command::Dist(DistArgs {
                what: DistCommand::Desktop(a),
            }),
        }),
        Command::Dist(d) => match d.what {
            DistCommand::Web(a) => dist::web(&dist::WebOptions {
                base: a.base,
                target: a.target,
            })
            .map(drop),
            DistCommand::Desktop(a) => dist::desktop(&dist::DesktopOptions {
                target: a.target,
                updater: a.updater,
                bundles: a.bundles,
                no_strip: a.no_strip,
            })
            .map(drop),
            DistCommand::All(a) => {
                dist::web(&dist::WebOptions {
                    base: a.base,
                    target: None,
                })?;
                dist::desktop(&dist::DesktopOptions {
                    target: a.desktop_target,
                    updater: a.updater,
                    bundles: None,
                    no_strip: false,
                })?;
                util::step(format!(
                    "All artifacts are in {}",
                    util::rel(&util::release_dir())
                ));
                Ok(())
            }
        },
        Command::Check(a) => check::run(&check::CheckOptions {
            no_wasm: a.no_wasm,
            no_test: a.no_test,
            no_trunk: a.no_trunk,
            no_desktop: a.no_desktop,
            quick: a.quick,
        }),
        Command::CheckClasses(a) => classes::check_dist(
            &a.dist
                .unwrap_or_else(|| util::root().join(classes::DEFAULT_DIST)),
            a.list,
        ),
        Command::CleanRelease => dist::clean_release(),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if util::in_github_actions() {
                eprintln!("::error::{}", format!("{err:#}").replace('\n', "%0A"));
            }
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn cli_is_well_formed() {
        super::Cli::command().debug_assert();
    }
}
