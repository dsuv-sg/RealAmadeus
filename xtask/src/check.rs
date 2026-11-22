//! `cargo xtask check` — the same gates as .github/workflows/ci.yml, runnable locally.

use std::process::Command;

use anyhow::Result;

use crate::{classes, util};

/// Crates that run in the browser and must also pass clippy for `wasm32-unknown-unknown`.
pub const WASM_CRATES: [&str; 4] = [
    "amadeus-core",
    "amadeus-live2d",
    "amadeus-ui",
    "amadeus-web",
];

pub struct CheckOptions {
    pub no_wasm: bool,
    pub no_test: bool,
    pub no_trunk: bool,
    pub no_desktop: bool,
    /// Debug Trunk builds (faster; skips wasm-opt).
    pub quick: bool,
}

fn locked(cmd: &mut Command) -> &mut Command {
    if util::in_ci() {
        cmd.arg("--locked");
    }
    cmd
}

fn trunk_build(release: bool, extra: &[&str]) -> Result<()> {
    let mut cmd = util::command("trunk");
    cmd.current_dir(util::root().join("apps/web")).arg("build");
    if release {
        cmd.arg("--release");
    }
    cmd.args(extra);
    util::run(&mut cmd)
}

pub fn run(opts: &CheckOptions) -> Result<()> {
    util::step("cargo fmt --check");
    util::run(util::cargo().args(["fmt", "--all", "--", "--check"]))?;

    // Native checks cover the workspace `default-members` (everything except the desktop
    // app, which needs the Trunk-built frontend, and developer tools).
    util::step("clippy (native, default members)");
    util::run(
        locked(util::cargo().args(["clippy", "--all-targets"])).args(["--", "-D", "warnings"]),
    )?;

    if !opts.no_wasm {
        util::step("clippy (wasm32-unknown-unknown)");
        let mut cmd = util::cargo();
        cmd.args(["clippy", "--target", "wasm32-unknown-unknown"]);
        for krate in WASM_CRATES {
            cmd.args(["-p", krate]);
        }
        util::run(locked(&mut cmd).args(["--", "-D", "warnings"]))?;
    }

    if !opts.no_test {
        util::step("cargo test (default members)");
        util::run(locked(util::cargo().arg("test")))?;
    }

    if !opts.no_trunk {
        util::step("Trunk build (web)");
        trunk_build(!opts.quick, &[])?;
        // PAGE-051: a class Tailwind did not generate (built with format!, rejected value) only
        // shows up as an unstyled element, so compare the sources with the CSS just built.
        util::step("Tailwind arbitrary-value classes in the built CSS");
        classes::check_dist(&util::root().join(classes::DEFAULT_DIST), false)?;
    }

    if !opts.no_desktop {
        util::step("Trunk build (desktop frontend) + clippy for the desktop app");
        // The desktop crate embeds apps/web/dist-desktop at compile time.
        trunk_build(
            !opts.quick,
            &["--public-url", "/", "--dist", "dist-desktop"],
        )?;
        for features in [None, Some("updater")] {
            let mut cmd = util::cargo();
            cmd.args(["clippy", "-p", "amadeus-desktop", "--all-targets"]);
            if let Some(f) = features {
                cmd.args(["--features", f]);
            }
            util::run(locked(&mut cmd).args(["--", "-D", "warnings"]))?;
        }
    }

    util::step("All checks passed");
    Ok(())
}
