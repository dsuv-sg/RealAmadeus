//! GPU selection for the webview.
//!
//! The UI is a WebGL/Canvas heavy page (Live2D), so the webview must render on the fastest GPU of
//! the machine. On a laptop or PC with an integrated GPU *and* a discrete one (NVIDIA, AMD, Intel
//! Arc) the operating system usually starts the app on the integrated GPU to save power, which
//! makes the app heavy. This module asks for the discrete GPU:
//!
//! * **Windows (WebView2)**: Chromium switches (`--force-high-performance-gpu`, GPU
//!   rasterization, ignore the GPU blocklist) passed through the window's additional browser
//!   arguments, see [`windows_browser_args`].
//! * **Linux (WebKitGTK)**: PRIME render offload through the environment variables Mesa
//!   (`DRI_PRIME`) and the NVIDIA proprietary driver (`__NV_PRIME_RENDER_OFFLOAD`, ...) read, only
//!   when a faster GPU than the boot GPU exists ([`pick_offload`]), plus WebKit's hardware
//!   acceleration policy `Always` ([`enable_webkit_acceleration`]).
//! * **macOS (WKWebView)**: nothing to do; WebKit already uses the GPU (Apple Silicon has one GPU).
//!
//! Everything honours the user's own settings: variables that are already set are never
//! overwritten, and `AMADEUS_GPU=off` (or `0`, `false`, `no`) disables all of it.

// Each platform uses only its own part of this module.
#![allow(dead_code)]

/// Environment variable that turns the GPU tuning off (`off`, `0`, `false`, `no`).
pub const GPU_ENV: &str = "AMADEUS_GPU";

/// Whether the user turned the GPU tuning off.
pub fn disabled() -> bool {
    disabled_value(std::env::var(GPU_ENV).ok().as_deref())
}

fn disabled_value(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "off" | "0" | "false" | "no"
        )
    })
}

/// Arguments for WebView2. They replace wry's defaults, so those (the `--disable-features` list
/// and the autoplay policy that voice playback needs) are repeated here.
pub fn windows_browser_args() -> String {
    let mut args = String::from(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
         --autoplay-policy=no-user-gesture-required",
    );
    if !disabled() {
        args.push_str(
            " --force_high_performance_gpu --enable-gpu-rasterization --ignore-gpu-blocklist",
        );
    }
    args
}

/// Vendor classes ordered by how likely the GPU is a fast one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Vendor {
    Other,
    Intel,
    Amd,
    Nvidia,
}

impl Vendor {
    pub fn from_pci_id(id: u32) -> Self {
        match id {
            0x10de => Self::Nvidia,
            0x1002 => Self::Amd,
            0x8086 => Self::Intel,
            _ => Self::Other,
        }
    }
}

/// One display adapter as seen in `/sys/class/drm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adapter {
    pub vendor: Vendor,
    /// The firmware's boot display: the GPU the system uses by default.
    pub boot: bool,
    /// Dedicated video memory in bytes when the driver reports it (amdgpu), else 0.
    pub vram: u64,
}

/// Which GPU to ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offload {
    /// A proprietary NVIDIA GPU: `__NV_PRIME_RENDER_OFFLOAD` and friends.
    Nvidia,
    /// A Mesa-driven GPU (AMD, Intel Arc, nouveau): `DRI_PRIME=1`.
    Mesa,
}

/// Decides whether the app should be moved off the boot GPU. Returns `None` for a single GPU and
/// when the boot GPU already is the best one (a desktop PC whose monitor sits on the discrete
/// card: `DRI_PRIME=1` would pick the *integrated* GPU there).
pub fn pick_offload(adapters: &[Adapter], nvidia_proprietary: bool) -> Option<Offload> {
    let rank = |a: &Adapter| (a.vendor, a.vram);
    let boot = adapters.iter().find(|a| a.boot).or(adapters.first())?;
    let best = adapters
        .iter()
        .filter(|a| a.vendor != Vendor::Other)
        .max_by_key(|a| rank(a))?;
    if adapters.len() < 2 || rank(best) <= rank(boot) {
        return None;
    }
    Some(if best.vendor == Vendor::Nvidia && nvidia_proprietary {
        Offload::Nvidia
    } else {
        Offload::Mesa
    })
}

/// Called first thing in `run()`, before any thread or webview exists.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn prepare_linux() {
    if disabled() {
        log::info!("GPU tuning disabled by {GPU_ENV}");
        return;
    }
    let adapters = linux::adapters();
    let nvidia = std::path::Path::new("/proc/driver/nvidia/version").exists();
    log::info!("display adapters: {adapters:?} (NVIDIA proprietary driver: {nvidia})");
    let Some(offload) = pick_offload(&adapters, nvidia) else {
        return;
    };
    // Never override what the user or the launcher already chose.
    let set_default = |key: &str, value: &str| {
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    };
    match offload {
        Offload::Mesa => {
            if std::env::var_os("__NV_PRIME_RENDER_OFFLOAD").is_none() {
                set_default("DRI_PRIME", "1");
            }
        }
        Offload::Nvidia => {
            set_default("__NV_PRIME_RENDER_OFFLOAD", "1");
            set_default("__GLX_VENDOR_LIBRARY_NAME", "nvidia");
            set_default("__VK_LAYER_NV_optimus", "NVIDIA_only");
            // EGL (WebKitGTK on Wayland and recent X11 builds) needs glvnd's vendor file; only
            // point at it when it exists, otherwise EGL would find no driver at all.
            for dir in ["/usr/share/glvnd/egl_vendor.d", "/etc/glvnd/egl_vendor.d"] {
                let file = format!("{dir}/10_nvidia.json");
                if std::path::Path::new(&file).exists() {
                    set_default("__EGL_VENDOR_LIBRARY_FILENAMES", &file);
                    break;
                }
            }
        }
    }
    log::info!("requested the discrete GPU for rendering ({offload:?})");
}

/// WebKitGTK composites on the GPU only "on demand" by default; WebGL and animated canvases are
/// much smoother with `Always`.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn enable_webkit_acceleration(window: &tauri::WebviewWindow) {
    if disabled() {
        return;
    }
    let result = window.with_webview(|webview| {
        use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
        if let Some(settings) = webview.inner().settings() {
            settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Always);
        }
    });
    if let Err(err) = result {
        log::warn!("cannot enable WebKit hardware acceleration: {err}");
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux {
    use std::{fs, path::Path};

    use super::{Adapter, Vendor};

    fn read_u64(path: &Path, radix: u32) -> Option<u64> {
        let text = fs::read_to_string(path).ok()?;
        u64::from_str_radix(text.trim().trim_start_matches("0x"), radix).ok()
    }

    /// Every `/sys/class/drm/cardN` that is a real PCI display adapter.
    pub fn adapters() -> Vec<Adapter> {
        let Ok(entries) = fs::read_dir("/sys/class/drm") else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // `card0`, not `card0-HDMI-A-1` (connectors) or `renderD128`.
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }
            let device = entry.path().join("device");
            let Some(vendor) = read_u64(&device.join("vendor"), 16) else {
                continue;
            };
            found.push(Adapter {
                vendor: Vendor::from_pci_id(u32::try_from(vendor).unwrap_or(0)),
                boot: read_u64(&device.join("boot_vga"), 10) == Some(1),
                vram: read_u64(&device.join("mem_info_vram_total"), 10).unwrap_or(0),
            });
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn gpu(vendor: Vendor, boot: bool, vram: u64) -> Adapter {
        Adapter { vendor, boot, vram }
    }

    #[test]
    fn single_gpu_needs_nothing() {
        assert_eq!(pick_offload(&[gpu(Vendor::Intel, true, 0)], false), None);
        assert_eq!(pick_offload(&[], false), None);
    }

    #[test]
    fn intel_plus_nvidia_offloads_to_nvidia() {
        let a = [gpu(Vendor::Intel, true, 0), gpu(Vendor::Nvidia, false, 0)];
        assert_eq!(pick_offload(&a, true), Some(Offload::Nvidia));
        // nouveau: Mesa's DRI_PRIME.
        assert_eq!(pick_offload(&a, false), Some(Offload::Mesa));
    }

    #[test]
    fn amd_apu_plus_amd_dgpu_compares_vram() {
        let a = [
            gpu(Vendor::Amd, true, 512 << 20),
            gpu(Vendor::Amd, false, 8 << 30),
        ];
        assert_eq!(pick_offload(&a, false), Some(Offload::Mesa));
    }

    #[test]
    fn boot_gpu_that_is_already_the_best_is_left_alone() {
        let desktop = [gpu(Vendor::Nvidia, true, 0), gpu(Vendor::Intel, false, 0)];
        assert_eq!(pick_offload(&desktop, true), None);
        let amd = [
            gpu(Vendor::Amd, true, 8 << 30),
            gpu(Vendor::Amd, false, 512 << 20),
        ];
        assert_eq!(pick_offload(&amd, false), None);
    }

    #[test]
    fn off_switch() {
        assert!(disabled_value(Some("off")));
        assert!(disabled_value(Some(" 0 ")));
        assert!(disabled_value(Some("FALSE")));
        assert!(!disabled_value(Some("auto")));
        assert!(!disabled_value(None));
    }

    #[test]
    fn windows_args_keep_the_defaults() {
        let args = windows_browser_args();
        assert!(args.contains("--disable-features=msWebOOUI"));
        assert!(args.contains("--autoplay-policy=no-user-gesture-required"));
    }
}
