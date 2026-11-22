//! Release coordinates of RealAmadeus V2 (DECISIONS D2), shared by the download page
//! (`amadeus-ui`) and the release tooling so that links and uploaded file names cannot drift.
//!
//! * Repository: [`REPO_SLUG`] (`dsuv-sg/RealAmadeus`, the workspace `repository`).
//! * Tag: `v<semver>` ([`tag_for`], e.g. `v2.0.0`).
//! * Assets: `RealAmadeus_V<display>_<Platform>_<arch><suffix>` ([`AssetSpec::file_name`]).
//!   `<arch>` keeps each package format's own convention, exactly like `xtask`
//!   (`xtask/src/platform.rs`) and tauri-action's `releaseAssetNamePattern`: `x64` for Windows,
//!   `universal` for macOS, `amd64` for `.AppImage`/`.deb`, `x86_64` for `.rpm`.
//! * Every release also carries [`CHECKSUMS_FILE`].
//! * V1 downloads (Unity `U` and Qt `Q` editions, tag `RealAmadeusV1.4`) are in [`legacy_v1`].
//!
//! I/O-free: URLs are only built here, never fetched.

use crate::version::{format_display_version, PRODUCT_NAME};

/// GitHub `owner/name` of the V2 repository.
pub const REPO_SLUG: &str = "dsuv-sg/RealAmadeus";
/// V2 repository URL.
pub const REPO_URL: &str = "https://github.com/dsuv-sg/RealAmadeus";
/// List of all V2 releases.
pub const RELEASES_PAGE: &str = "https://github.com/dsuv-sg/RealAmadeus/releases";
/// SHA-256 list uploaded with every release.
pub const CHECKSUMS_FILE: &str = "SHA256SUMS.txt";
/// The hosted web version.
pub const WEB_URL: &str = "https://dsuv.elvelt.com/amadeus";

/// Git tag of a version: `v2.0.0`.
#[must_use]
pub fn tag_for(version: &str) -> String {
    format!("v{version}")
}

/// Common file-name prefix of a version's assets: `RealAmadeus_V2.0`.
#[must_use]
pub fn asset_prefix(version: &str) -> String {
    format!("{PRODUCT_NAME}_{}", format_display_version(version))
}

/// Download URL of a file attached to the release of `version`.
#[must_use]
pub fn download_url(version: &str, file: &str) -> String {
    format!(
        "https://github.com/{REPO_SLUG}/releases/download/{}/{file}",
        tag_for(version)
    )
}

/// Release page of `version` (`…/releases/tag/v2.0.0`).
#[must_use]
pub fn release_page_url(version: &str) -> String {
    format!("{RELEASES_PAGE}/tag/{}", tag_for(version))
}

/// Download URL of [`CHECKSUMS_FILE`] for `version`.
#[must_use]
pub fn checksums_url(version: &str) -> String {
    download_url(version, CHECKSUMS_FILE)
}

/// Platform of a release asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Windows 10/11.
    Windows,
    /// macOS (universal: Apple Silicon + Intel).
    MacOs,
    /// Linux.
    Linux,
    /// Self-hosted web server (frontend + `amadeus-server`).
    Web,
}

impl Platform {
    /// All platforms in display order.
    pub const ALL: [Platform; 4] = [
        Platform::Windows,
        Platform::MacOs,
        Platform::Linux,
        Platform::Web,
    ];

    /// The platform as written in asset names and headings.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Platform::Windows => "Windows",
            Platform::MacOs => "macOS",
            Platform::Linux => "Linux",
            Platform::Web => "Web",
        }
    }
}

/// Kind of a release asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssetKind {
    /// NSIS installer (`-setup.exe`).
    Nsis,
    /// WiX installer (`.msi`). Not built for pre-releases with non-numeric identifiers.
    Msi,
    /// Disk image (`.dmg`).
    Dmg,
    /// Zipped app bundle (`.app.tar.gz`, used by the updater).
    AppArchive,
    /// `.AppImage`.
    AppImage,
    /// Debian package (`.deb`).
    Deb,
    /// RPM package (`.rpm`).
    Rpm,
    /// Web server package (`.tar.gz`).
    WebServer,
}

/// One entry of the release asset table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetSpec {
    /// What the file is.
    pub kind: AssetKind,
    /// Platform it is for.
    pub platform: Platform,
    /// Architecture as written in the file name.
    pub arch: &'static str,
    /// Everything after the architecture (`-setup.exe`, `.msi`, …).
    pub suffix: &'static str,
    /// The primary download of its platform.
    pub recommended: bool,
}

impl AssetSpec {
    /// File name for `version`, e.g. `RealAmadeus_V2.0_Windows_x64-setup.exe`.
    #[must_use]
    pub fn file_name(&self, version: &str) -> String {
        format!(
            "{}_{}_{}{}",
            asset_prefix(version),
            self.platform.label(),
            self.arch,
            self.suffix
        )
    }

    /// Download URL of this asset in the release of `version`.
    #[must_use]
    pub fn url(&self, version: &str) -> String {
        download_url(version, &self.file_name(version))
    }

    /// Whether the release workflow builds this asset for `version` (MSI needs a numeric
    /// version, see [`msi_compatible`]).
    #[must_use]
    pub fn is_built_for(&self, version: &str) -> bool {
        self.kind != AssetKind::Msi || msi_compatible(version)
    }
}

/// The release assets of the x86-64 / universal builds, in display order (recommended first
/// within each platform). Same names as `cargo xtask dist` and the CI release workflow.
pub const ASSETS: [AssetSpec; 8] = [
    AssetSpec {
        kind: AssetKind::Nsis,
        platform: Platform::Windows,
        arch: "x64",
        suffix: "-setup.exe",
        recommended: true,
    },
    AssetSpec {
        kind: AssetKind::Msi,
        platform: Platform::Windows,
        arch: "x64",
        suffix: ".msi",
        recommended: false,
    },
    AssetSpec {
        kind: AssetKind::Dmg,
        platform: Platform::MacOs,
        arch: "universal",
        suffix: ".dmg",
        recommended: true,
    },
    AssetSpec {
        kind: AssetKind::AppArchive,
        platform: Platform::MacOs,
        arch: "universal",
        suffix: ".app.tar.gz",
        recommended: false,
    },
    AssetSpec {
        kind: AssetKind::AppImage,
        platform: Platform::Linux,
        arch: "amd64",
        suffix: ".AppImage",
        recommended: true,
    },
    AssetSpec {
        kind: AssetKind::Deb,
        platform: Platform::Linux,
        arch: "amd64",
        suffix: ".deb",
        recommended: false,
    },
    AssetSpec {
        kind: AssetKind::Rpm,
        platform: Platform::Linux,
        arch: "x86_64",
        suffix: ".rpm",
        recommended: false,
    },
    AssetSpec {
        kind: AssetKind::WebServer,
        platform: Platform::Web,
        arch: "Linux-x64",
        suffix: ".tar.gz",
        recommended: true,
    },
];

/// The table entry of `kind`.
#[must_use]
pub fn asset(kind: AssetKind) -> &'static AssetSpec {
    match ASSETS.iter().position(|spec| spec.kind == kind) {
        Some(index) => &ASSETS[index],
        // Every kind has exactly one entry (checked by the tests).
        None => &ASSETS[0],
    }
}

/// The assets of `platform`, in table order.
pub fn assets_for(platform: Platform) -> impl Iterator<Item = &'static AssetSpec> {
    ASSETS.iter().filter(move |spec| spec.platform == platform)
}

/// Inverse of [`AssetSpec::file_name`]: the table entry and the display version (without `V`)
/// of an asset file name (`RealAmadeus_V2.0_Linux_amd64.deb` → (`Deb`, `"2.0"`),
/// `RealAmadeus_V2.1.3_Linux_amd64.deb` → (`Deb`, `"2.1.3"`)).
#[must_use]
pub fn classify(file: &str) -> Option<(&'static AssetSpec, &str)> {
    let rest = file.strip_prefix(PRODUCT_NAME)?.strip_prefix("_V")?;
    ASSETS.iter().find_map(|spec| {
        let tail = format!("_{}_{}{}", spec.platform.label(), spec.arch, spec.suffix);
        let version = rest.strip_suffix(tail.as_str())?;
        (!version.is_empty()).then_some((spec, version))
    })
}

/// Whether Tauri's WiX bundler accepts `version` (same rule as `xtask`'s `msi_compatible`):
/// `major.minor.patch` with major/minor ≤ 255 and patch ≤ 65535, plus at most one numeric
/// pre-release identifier and numeric build metadata, each ≤ 65535 (`2.1.0-1` yes,
/// `2.1.0-rc.1` no).
#[must_use]
pub fn msi_compatible(version: &str) -> bool {
    let (rest, build) = match version.split_once('+') {
        Some((rest, build)) => (rest, Some(build)),
        None => (version, None),
    };
    let (core, pre) = match rest.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (rest, None),
    };
    let number = |s: &str, max: u64| {
        !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit())
            && s.parse::<u64>().is_ok_and(|n| n <= max)
    };
    let mut parts = core.split('.');
    let core_ok = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(major), Some(minor), Some(patch), None) => {
            number(major, 255) && number(minor, 255) && number(patch, 65_535)
        }
        _ => false,
    };
    let pre_ok = pre.is_none_or(|pre| number(pre, 65_535));
    let build_ok = build.is_none_or(|build| number(build, 65_535));
    core_ok && pre_ok && build_ok
}

/// V1 (`V1.4U` Unity and `V1.4Q` Qt) legacy download coordinates, kept for the legacy section of
/// the download page and the docs.
pub mod legacy_v1 {
    /// V1 (Unity) repository.
    pub const REPO_URL: &str = "https://github.com/dsuv-sg/RealAmadeusUnity";
    /// V1 (Unity) releases page.
    pub const RELEASES_U: &str = "https://github.com/dsuv-sg/RealAmadeusUnity/releases";
    /// V1 (Qt) releases page.
    pub const RELEASES_Q: &str = "https://github.com/dsuv-sg/RealAmadeusQT/releases";
    /// V1.4U Windows build.
    pub const WIN_U: &str = "https://github.com/dsuv-sg/RealAmadeusUnity/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_Win64.zip";
    /// V1.4U macOS build.
    pub const MAC_U: &str = "https://github.com/dsuv-sg/RealAmadeusUnity/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_MacUniversal.zip";
    /// V1.4U Linux build.
    pub const LINUX_U: &str = "https://github.com/dsuv-sg/RealAmadeusUnity/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_Linux.zip";
    /// V1.4Q (lightweight) Windows build.
    pub const WIN_Q: &str = "https://github.com/dsuv-sg/RealAmadeusQT/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4Q_Win64.zip";
    /// Badge of the Unity edition cards.
    pub const BADGE_U: &str = "V1.4U";
    /// Badge of the Qt edition card.
    pub const BADGE_Q: &str = "V1.4Q";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_match_the_release_workflow() {
        // docs/RELEASE.md and xtask/src/platform.rs (x86-64 / universal targets).
        let names: Vec<String> = ASSETS.iter().map(|spec| spec.file_name("2.0.0")).collect();
        assert_eq!(
            names,
            [
                "RealAmadeus_V2.0_Windows_x64-setup.exe",
                "RealAmadeus_V2.0_Windows_x64.msi",
                "RealAmadeus_V2.0_macOS_universal.dmg",
                "RealAmadeus_V2.0_macOS_universal.app.tar.gz",
                "RealAmadeus_V2.0_Linux_amd64.AppImage",
                "RealAmadeus_V2.0_Linux_amd64.deb",
                "RealAmadeus_V2.0_Linux_x86_64.rpm",
                "RealAmadeus_V2.0_Web_Linux-x64.tar.gz",
            ]
        );
    }

    #[test]
    fn urls_use_the_v_tag() {
        assert_eq!(tag_for("2.0.0"), "v2.0.0");
        assert_eq!(
            asset(AssetKind::Nsis).url("2.0.0"),
            "https://github.com/dsuv-sg/RealAmadeus/releases/download/v2.0.0/RealAmadeus_V2.0_Windows_x64-setup.exe"
        );
        assert_eq!(
            checksums_url("2.0.1"),
            "https://github.com/dsuv-sg/RealAmadeus/releases/download/v2.0.1/SHA256SUMS.txt"
        );
        assert_eq!(
            release_page_url("2.0.0"),
            "https://github.com/dsuv-sg/RealAmadeus/releases/tag/v2.0.0"
        );
        assert!(REPO_URL.ends_with(REPO_SLUG));
        assert!(RELEASES_PAGE.starts_with(REPO_URL));
    }

    #[test]
    fn every_kind_has_one_entry_and_each_platform_one_recommended() {
        for spec in &ASSETS {
            assert_eq!(asset(spec.kind), spec);
            assert_eq!(ASSETS.iter().filter(|s| s.kind == spec.kind).count(), 1);
        }
        for platform in Platform::ALL {
            let specs: Vec<_> = assets_for(platform).collect();
            assert!(!specs.is_empty());
            assert_eq!(specs.iter().filter(|s| s.recommended).count(), 1);
            assert!(
                specs[0].recommended,
                "{platform:?}: recommended asset first"
            );
        }
    }

    #[test]
    fn classify_round_trips() {
        for version in ["2.0.0", "2.1.3", "2.1.0-rc.1", "10.20.30-1+7"] {
            let display = format_display_version(version);
            let expected = display.strip_prefix('V').unwrap();
            for spec in &ASSETS {
                let name = spec.file_name(version);
                assert_eq!(classify(&name), Some((spec, expected)), "{name}");
            }
        }
        assert_eq!(classify("RealAmadeus_V1.4U_Win64.zip"), None);
        assert_eq!(classify("SHA256SUMS.txt"), None);
        assert_eq!(classify("RealAmadeus_V_Linux_amd64.deb"), None);
    }

    #[test]
    fn msi_only_for_numeric_versions() {
        for ok in ["2.0.0", "2.1.0-1", "255.255.65535", "2.0.0+7"] {
            assert!(msi_compatible(ok), "{ok}");
        }
        for bad in [
            "2.1.0-rc.1",
            "2.1.0-rc1",
            "256.0.0",
            "2.0",
            "2.0.0.0",
            "2.0.0-1.2",
            "2.0.0+abc",
            "",
        ] {
            assert!(!msi_compatible(bad), "{bad}");
        }
        assert!(asset(AssetKind::Msi).is_built_for("2.0.0"));
        assert!(!asset(AssetKind::Msi).is_built_for("2.1.0-rc.1"));
        assert!(asset(AssetKind::Nsis).is_built_for("2.1.0-rc.1"));
    }

    #[test]
    fn legacy_urls_are_v1_verbatim() {
        assert_eq!(
            legacy_v1::REPO_URL,
            "https://github.com/dsuv-sg/RealAmadeusUnity"
        );
        assert!(
            legacy_v1::WIN_U.starts_with("https://github.com/dsuv-sg/RealAmadeusUnity/releases/")
        );
        assert!(legacy_v1::WIN_Q.ends_with("RealAmadeus_V1.4Q_Win64.zip"));
        assert!(legacy_v1::MAC_U.ends_with("RealAmadeus_V1.4U_MacUniversal.zip"));
        assert!(legacy_v1::LINUX_U.ends_with("RealAmadeus_V1.4U_Linux.zip"));
    }
}
