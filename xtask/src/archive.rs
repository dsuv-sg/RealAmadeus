//! `.tar.gz` / `.zip` creation through the platform's archivers, so xtask needs no
//! compression crates:
//!
//! * Linux / macOS: `tar` (GNU tar or bsdtar) and, for zip, Info-ZIP `zip` or `bsdtar`.
//! * Windows 10+: `%SystemRoot%\System32\tar.exe` (bsdtar) writes both formats. It is used
//!   explicitly because Git for Windows puts GNU tar (which cannot write zip and treats `C:` as
//!   a remote host) first on PATH in `bash` shells.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};

use crate::util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    TarGz,
    Zip,
}

impl Format {
    pub fn from_ext(ext: &str) -> Result<Format> {
        match ext {
            "tar.gz" | "tgz" => Ok(Format::TarGz),
            "zip" => Ok(Format::Zip),
            other => bail!("unsupported archive format `{other}`"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Flavor {
    Gnu,
    Bsd,
}

struct Tar {
    program: OsString,
    flavor: Flavor,
}

fn windows_bsdtar() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    let path = PathBuf::from(root).join("System32").join("tar.exe");
    path.is_file().then_some(path)
}

fn find_tar() -> Result<Tar> {
    let candidates: Vec<OsString> = windows_bsdtar()
        .map(PathBuf::into_os_string)
        .into_iter()
        .chain(["tar".into(), "bsdtar".into(), "gtar".into()])
        .collect();
    for program in candidates {
        if let Some(version) = util::probe(util::command(&program).arg("--version")) {
            let flavor = if version.contains("GNU tar") {
                Flavor::Gnu
            } else {
                Flavor::Bsd
            };
            return Ok(Tar { program, flavor });
        }
    }
    bail!(
        "no `tar` executable found. A tar is required to build release archives \
         (Windows 10+ ships %SystemRoot%\\System32\\tar.exe; on Linux install GNU tar)."
    )
}

/// Packs `parent/dir_name` (as a top-level directory) into `out`.
pub fn create(parent: &Path, dir_name: &str, out: &Path, format: Format) -> Result<()> {
    if !parent.join(dir_name).is_dir() {
        bail!("{} does not exist", parent.join(dir_name).display());
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
    }
    util::remove_file_if_exists(out)?;
    match format {
        Format::TarGz => tar_gz(parent, dir_name, out),
        Format::Zip => zip(parent, dir_name, out),
    }
}

fn tar_gz(parent: &Path, dir_name: &str, out: &Path) -> Result<()> {
    let tar = find_tar()?;
    let mut cmd = util::command(&tar.program);
    if tar.flavor == Flavor::Gnu {
        // Reproducible-ish: no builder user names, stable member order.
        cmd.args(["--owner=0", "--group=0", "--numeric-owner", "--sort=name"]);
        if cfg!(windows) {
            cmd.arg("--force-local");
        }
    }
    cmd.arg("-czf").arg(out).arg("-C").arg(parent).arg(dir_name);
    // macOS bsdtar would otherwise add AppleDouble `._*` files for extended attributes.
    cmd.env("COPYFILE_DISABLE", "1");
    util::run(&mut cmd)
}

fn zip(parent: &Path, dir_name: &str, out: &Path) -> Result<()> {
    // bsdtar picks the zip format from the `.zip` suffix with `-a`.
    let bsd = find_tar().ok().filter(|t| t.flavor == Flavor::Bsd);
    if let Some(tar) = bsd {
        let mut cmd = util::command(&tar.program);
        cmd.arg("-a")
            .arg("-cf")
            .arg(out)
            .arg("-C")
            .arg(parent)
            .arg(dir_name)
            .env("COPYFILE_DISABLE", "1");
        return util::run(&mut cmd);
    }
    if util::has_program("zip", &["-v"]) {
        let mut cmd = Command::new("zip");
        cmd.current_dir(parent)
            .args(["-r", "-q", "-X", "-y"])
            .arg(out)
            .arg(dir_name);
        return util::run(&mut cmd);
    }
    bail!("creating a .zip needs bsdtar or Info-ZIP `zip`; install one of them (e.g. `sudo apt install zip`)")
}
