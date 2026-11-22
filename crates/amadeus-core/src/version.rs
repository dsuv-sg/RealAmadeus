//! Version information. The workspace `Cargo.toml` is the single source of truth.

/// Semantic version of this build, e.g. `2.0.0`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Product name used for installers and release assets.
pub const PRODUCT_NAME: &str = "RealAmadeus";

/// Edition suffix. V1 used `U` (Unity) and `Q` (Qt); V2 drops the edition suffix.
pub const EDITION_SUFFIX: &str = "";

/// Major generation of the product line.
pub const GENERATION: &str = "V2";

/// Format a version string for display, dropping a `.0` patch on release versions (`2.0.0` → `V2.0`).
#[must_use]
pub fn format_display_version(version: &str) -> String {
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

/// Human readable version, e.g. `V2.0`.
#[must_use]
pub fn display_version() -> String {
    format_display_version(VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_version_formats_release_without_patch_zero_or_suffix() {
        assert_eq!(display_version(), "V2.0");
        assert_eq!(format_display_version("2.0.0"), "V2.0");
        assert_eq!(format_display_version("2.1.0"), "V2.1");
        assert_eq!(format_display_version("2.0.1"), "V2.0.1");
        assert_eq!(format_display_version("2.1.0-rc.1"), "V2.1.0-rc.1");
        assert_eq!(format_display_version("2.0.0+7"), "V2.0.0+7");
    }
}
