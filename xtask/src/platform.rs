//! Target triples and release asset names.
//!
//! Asset names follow `RealAmadeus_V<display>_<Platform>_<arch>[-setup]<ext>`. The CI release
//! workflow uploads desktop bundles through tauri-action with
//! `releaseAssetNamePattern: RealAmadeus_<label>_<Platform>_[arch][setup][ext]`, so the
//! `<arch>` values below mirror tauri-action's `[arch]` (each package format keeps its native
//! architecture name: `amd64` for .deb/.AppImage, `x86_64` for .rpm, `x64` for Windows/macOS).
//! Local `cargo xtask dist desktop` builds therefore produce byte-identical file names.

use anyhow::{bail, Result};

use crate::util::PRODUCT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Mac,
    Linux,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub triple: String,
    /// First component of the triple (`x86_64`, `aarch64`, `i686`, `armv7`, `universal`).
    pub arch: String,
    pub os: Os,
}

/// Tauri bundle formats (the sub-directories of `target/**/release/bundle/`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bundle {
    Nsis,
    Msi,
    Dmg,
    App,
    AppImage,
    Deb,
    Rpm,
}

impl Bundle {
    pub const ALL: [Bundle; 7] = [
        Bundle::Nsis,
        Bundle::Msi,
        Bundle::Dmg,
        Bundle::App,
        Bundle::AppImage,
        Bundle::Deb,
        Bundle::Rpm,
    ];

    /// Directory under `release/bundle/`.
    pub fn dir(self) -> &'static str {
        match self {
            Bundle::Nsis => "nsis",
            Bundle::Msi => "msi",
            Bundle::Dmg => "dmg",
            Bundle::App => "macos",
            Bundle::AppImage => "appimage",
            Bundle::Deb => "deb",
            Bundle::Rpm => "rpm",
        }
    }

    /// File suffixes produced for this bundle, longest first (installer, updater archive and
    /// their minisign signatures). Same list and order as tauri-action.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Bundle::Nsis => &[".nsis.zip.sig", ".nsis.zip", ".exe.sig", ".exe"],
            Bundle::Msi => &[".msi.zip.sig", ".msi.zip", ".msi.sig", ".msi"],
            Bundle::Dmg => &[".dmg"],
            Bundle::App => &[".app.tar.gz.sig", ".app.tar.gz"],
            Bundle::AppImage => &[
                ".AppImage.tar.gz.sig",
                ".AppImage.tar.gz",
                ".AppImage.sig",
                ".AppImage",
            ],
            Bundle::Deb => &[".deb.sig", ".deb"],
            Bundle::Rpm => &[".rpm.sig", ".rpm"],
        }
    }

    pub fn os(self) -> Os {
        match self {
            Bundle::Nsis | Bundle::Msi => Os::Windows,
            Bundle::Dmg | Bundle::App => Os::Mac,
            Bundle::AppImage | Bundle::Deb | Bundle::Rpm => Os::Linux,
        }
    }
}

/// The installer suffix of a bundle (what users download).
pub fn main_ext(bundle: Bundle) -> &'static str {
    match bundle {
        Bundle::Nsis => ".exe",
        Bundle::Msi => ".msi",
        Bundle::Dmg => ".dmg",
        Bundle::App => ".app.tar.gz",
        Bundle::AppImage => ".AppImage",
        Bundle::Deb => ".deb",
        Bundle::Rpm => ".rpm",
    }
}

impl Target {
    pub fn parse(triple: &str) -> Result<Target> {
        let triple = triple.trim();
        let arch = triple.split('-').next().unwrap_or_default().to_string();
        let os = if triple.contains("windows") {
            Os::Windows
        } else if triple.contains("apple-darwin") {
            Os::Mac
        } else if triple.contains("linux") {
            Os::Linux
        } else {
            bail!(
                "unsupported target `{triple}` (supported: *-pc-windows-msvc, *-apple-darwin, \
                 universal-apple-darwin, *-unknown-linux-gnu)"
            );
        };
        if arch.is_empty() || !triple.contains('-') {
            bail!("`{triple}` is not a target triple");
        }
        Ok(Target {
            triple: triple.to_string(),
            arch,
            os,
        })
    }

    pub fn platform_label(&self) -> &'static str {
        match self.os {
            Os::Windows => "Windows",
            Os::Mac => "macOS",
            Os::Linux => "Linux",
        }
    }

    /// Generic architecture label used for the web package (`x64`, `arm64`, ...).
    pub fn generic_arch(&self) -> String {
        match self.arch.as_str() {
            "x86_64" => "x64".into(),
            "aarch64" => "arm64".into(),
            a if is_x86_32(a) => "x86".into(),
            a => a.into(),
        }
    }

    pub fn exe_suffix(&self) -> &'static str {
        if self.os == Os::Windows {
            ".exe"
        } else {
            ""
        }
    }

    /// Archive format of the web package: `.zip` for Windows, `.tar.gz` elsewhere.
    pub fn archive_ext(&self) -> &'static str {
        if self.os == Os::Windows {
            "zip"
        } else {
            "tar.gz"
        }
    }

    /// Architecture label of a desktop bundle — identical to tauri-action's `[arch]`.
    pub fn bundle_arch(&self, bundle: Bundle) -> String {
        let a = self.arch.as_str();
        let s = match bundle {
            Bundle::Dmg | Bundle::App => match a {
                "x86_64" => "x64",
                "aarch64" => "aarch64",
                other => other,
            },
            Bundle::Nsis | Bundle::Msi => {
                if is_x86_32(a) {
                    "x86"
                } else if a == "aarch64" {
                    "arm64"
                } else {
                    "x64"
                }
            }
            Bundle::Deb => match a {
                "x86_64" => "amd64",
                "aarch64" => "arm64",
                "armv7" | "arm" => "armhf",
                other if is_x86_32(other) => "i386",
                other => other,
            },
            Bundle::Rpm => match a {
                "armv7" | "arm" => "armhfp",
                other if is_x86_32(other) => "i386",
                other => other,
            },
            Bundle::AppImage => match a {
                "x86_64" => "amd64",
                "armv7" => "arm",
                other if is_x86_32(other) => "i386",
                other => other,
            },
        };
        s.to_string()
    }
}

fn is_x86_32(arch: &str) -> bool {
    matches!(arch, "i686" | "i586" | "i386" | "x86")
}

/// `RealAmadeus_V2.0`
pub fn asset_prefix(version: &str) -> String {
    format!("{PRODUCT}_{}", crate::version::format_label(version))
}

/// `RealAmadeus_V2.0_Linux_amd64.deb`, `RealAmadeus_V2.0_Windows_x64-setup.exe`, ...
pub fn bundle_asset_name(version: &str, target: &Target, bundle: Bundle, ext: &str) -> String {
    let setup = if bundle == Bundle::Nsis { "-setup" } else { "" };
    format!(
        "{}_{}_{}{setup}{ext}",
        asset_prefix(version),
        target.platform_label(),
        target.bundle_arch(bundle)
    )
}

/// `RealAmadeus_V2.0_Web_Linux-x64` (archive / top-level directory name of the web package).
pub fn web_package_name(version: &str, target: &Target) -> String {
    format!(
        "{}_Web_{}-{}",
        asset_prefix(version),
        target.platform_label(),
        target.generic_arch()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Target {
        Target::parse(s).unwrap()
    }

    /// The download page (`amadeus_core::release::ASSETS`, used by amadeus-ui) and xtask must
    /// produce the same file names, or the page links to assets that do not exist.
    #[test]
    fn names_match_the_download_page_table() {
        use amadeus_core::release::{AssetKind, ASSETS};
        let v = "2.0.0";
        for spec in ASSETS.iter() {
            let ours = match spec.kind {
                AssetKind::Nsis => {
                    bundle_asset_name(v, &t("x86_64-pc-windows-msvc"), Bundle::Nsis, ".exe")
                }
                AssetKind::Msi => {
                    bundle_asset_name(v, &t("x86_64-pc-windows-msvc"), Bundle::Msi, ".msi")
                }
                AssetKind::Dmg => {
                    bundle_asset_name(v, &t("universal-apple-darwin"), Bundle::Dmg, ".dmg")
                }
                AssetKind::AppArchive => {
                    bundle_asset_name(v, &t("universal-apple-darwin"), Bundle::App, ".app.tar.gz")
                }
                AssetKind::AppImage => bundle_asset_name(
                    v,
                    &t("x86_64-unknown-linux-gnu"),
                    Bundle::AppImage,
                    ".AppImage",
                ),
                AssetKind::Deb => {
                    bundle_asset_name(v, &t("x86_64-unknown-linux-gnu"), Bundle::Deb, ".deb")
                }
                AssetKind::Rpm => {
                    bundle_asset_name(v, &t("x86_64-unknown-linux-gnu"), Bundle::Rpm, ".rpm")
                }
                AssetKind::WebServer => format!(
                    "{}.{}",
                    web_package_name(v, &t("x86_64-unknown-linux-gnu")),
                    t("x86_64-unknown-linux-gnu").archive_ext()
                ),
            };
            assert_eq!(ours, spec.file_name(v), "{:?}", spec.kind);
        }
        assert_eq!(asset_prefix(v), amadeus_core::release::asset_prefix(v));
    }

    #[test]
    fn parses_triples() {
        assert_eq!(t("x86_64-pc-windows-msvc").os, Os::Windows);
        assert_eq!(t("universal-apple-darwin").arch, "universal");
        assert_eq!(t("aarch64-unknown-linux-gnu").os, Os::Linux);
        assert!(Target::parse("wasm32-unknown-unknown").is_err());
        assert!(Target::parse("linux").is_err());
    }

    #[test]
    fn desktop_names_match_the_documented_scheme() {
        let v = "2.0.0";
        let win = t("x86_64-pc-windows-msvc");
        assert_eq!(
            bundle_asset_name(v, &win, Bundle::Nsis, ".exe"),
            "RealAmadeus_V2.0_Windows_x64-setup.exe"
        );
        assert_eq!(
            bundle_asset_name(v, &win, Bundle::Nsis, ".exe.sig"),
            "RealAmadeus_V2.0_Windows_x64-setup.exe.sig"
        );
        assert_eq!(
            bundle_asset_name(v, &win, Bundle::Msi, ".msi"),
            "RealAmadeus_V2.0_Windows_x64.msi"
        );
        assert_eq!(
            bundle_asset_name(v, &t("aarch64-pc-windows-msvc"), Bundle::Msi, ".msi"),
            "RealAmadeus_V2.0_Windows_arm64.msi"
        );

        let mac = t("universal-apple-darwin");
        assert_eq!(
            bundle_asset_name(v, &mac, Bundle::Dmg, ".dmg"),
            "RealAmadeus_V2.0_macOS_universal.dmg"
        );
        assert_eq!(
            bundle_asset_name(v, &mac, Bundle::App, ".app.tar.gz"),
            "RealAmadeus_V2.0_macOS_universal.app.tar.gz"
        );
        assert_eq!(
            bundle_asset_name(v, &t("x86_64-apple-darwin"), Bundle::Dmg, ".dmg"),
            "RealAmadeus_V2.0_macOS_x64.dmg"
        );

        let linux = t("x86_64-unknown-linux-gnu");
        assert_eq!(
            bundle_asset_name(v, &linux, Bundle::AppImage, ".AppImage"),
            "RealAmadeus_V2.0_Linux_amd64.AppImage"
        );
        assert_eq!(
            bundle_asset_name(v, &linux, Bundle::Deb, ".deb"),
            "RealAmadeus_V2.0_Linux_amd64.deb"
        );
        assert_eq!(
            bundle_asset_name(v, &linux, Bundle::Rpm, ".rpm"),
            "RealAmadeus_V2.0_Linux_x86_64.rpm"
        );

        let arm = t("aarch64-unknown-linux-gnu");
        assert_eq!(arm.bundle_arch(Bundle::AppImage), "aarch64");
        assert_eq!(arm.bundle_arch(Bundle::Deb), "arm64");
        assert_eq!(arm.bundle_arch(Bundle::Rpm), "aarch64");
    }

    #[test]
    fn web_names() {
        assert_eq!(
            web_package_name("2.0.0", &t("x86_64-unknown-linux-gnu")),
            "RealAmadeus_V2.0_Web_Linux-x64"
        );
        assert_eq!(
            web_package_name("2.0.0", &t("aarch64-apple-darwin")),
            "RealAmadeus_V2.0_Web_macOS-arm64"
        );
        assert_eq!(t("x86_64-pc-windows-msvc").archive_ext(), "zip");
        assert_eq!(t("x86_64-unknown-linux-gnu").archive_ext(), "tar.gz");
    }
}
