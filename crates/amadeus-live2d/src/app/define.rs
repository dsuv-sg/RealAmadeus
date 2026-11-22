//! Sample-app constants — V1 `lappdefine.ts` as deployed (pipeline spec §2.7) plus the values the
//! customized V1 layer hard-codes, in V2 form (resource URLs are base-relative, D1).

/// Default resources prefix, resolved against `document.baseURI` (V1: origin-absolute `/resources/`).
pub const RESOURCES_BASE: &str = "resources/";
/// Model directory under [`RESOURCES_BASE`] (V1 `ResourcesPath` + `ModelDir[0]` + `"/"`).
pub const MODEL_DIR: &str = "models/AmadeusKurisu5.0/reama5.0/";
/// V1 `ModelJsonFileName`.
pub const MODEL_JSON_FILE_NAME: &str = "Live2D紅莉栖forSDK5.0.model3.json";
/// The binary files V1's `preloadDefaultAssets` fetched (hard-coded on purpose so preloading needs no
/// model3.json round trip; they duplicate the model3.json references).
pub const PRELOAD_BINARIES: [&str; 3] = [
    MODEL_JSON_FILE_NAME,
    "Live2D紅莉栖forSDK5.0.moc3",
    "Live2D紅莉栖forSDK5.0.physics3.json",
];
/// The texture V1's `preloadDefaultAssets` decoded.
pub const PRELOAD_IMAGE: &str = "Live2D紅莉栖forSDK5.0.8192/texture_00.png";

/// Marker of the Core `<script>` tag (`data-core="true"`, the same attribute V1's Live2DCanvas used,
/// so a page never loads the Core twice).
pub const CORE_SCRIPT_MARKER_ATTR: &str = "data-core";
/// How long [`super::core_loader::ensure_core_loaded`] waits for the Core's WASM after its script
/// loaded (D7).
pub const CORE_READY_TIMEOUT_MS: f64 = 10_000.0;
/// Poll interval while waiting for the Core.
pub const CORE_READY_POLL_MS: i32 = 5;
/// How long an image load waits for `img.decode()` before uploading anyway (the upload then
/// decodes synchronously; same pixels). V1 waited without a limit.
pub const IMAGE_DECODE_WAIT_MS: i32 = 3000;

// --- LAppView (pipeline spec §8.1) --------------------------------------------------------------
pub const VIEW_SCALE: f64 = 1.0;
pub const VIEW_MAX_SCALE: f64 = 2.0;
pub const VIEW_MIN_SCALE: f64 = 0.8;
pub const VIEW_LOGICAL_LEFT: f64 = -1.0;
pub const VIEW_LOGICAL_RIGHT: f64 = 1.0;
pub const VIEW_LOGICAL_MAX_LEFT: f64 = -2.0;
pub const VIEW_LOGICAL_MAX_RIGHT: f64 = 2.0;
pub const VIEW_LOGICAL_MAX_BOTTOM: f64 = -2.0;
pub const VIEW_LOGICAL_MAX_TOP: f64 = 2.0;

// --- LAppLive2DManager.onUpdate upper-body zoom (pipeline spec §8.2, D8) -------------------------
/// Desktop (`canvas.width >= canvas.height`) relative scale.
pub const DESKTOP_SCALE: f64 = 4.1;
/// Desktop absolute translate Y.
pub const DESKTOP_TRANSLATE_Y: f64 = -2.75;
/// Portrait canvas (`canvas.width < canvas.height`) relative scale.
pub const MOBILE_SCALE: f64 = 3.3;
/// Portrait canvas absolute translate Y.
pub const MOBILE_TRANSLATE_Y: f64 = -2.1;

// --- motions / hit areas (latent for the shipped model) -----------------------------------------
pub const MOTION_GROUP_IDLE: &str = "Idle";
pub const MOTION_GROUP_TAP_BODY: &str = "TapBody";
pub const HIT_AREA_NAME_HEAD: &str = "Head";
pub const HIT_AREA_NAME_BODY: &str = "Body";

/// V1 `MOCConsistencyValidationEnable` (customized to `false`, spec §0.3 A2).
pub const MOC_CONSISTENCY_VALIDATION_ENABLE: bool = false;
/// V1 `MotionConsistencyValidationEnable` (customized to `false`).
pub const MOTION_CONSISTENCY_VALIDATION_ENABLE: bool = false;

/// `createRenderer(w, h, maskBufferCount = 1)`.
pub const MASK_BUFFER_COUNT: i32 = 1;
/// High-resolution clipping mask atlas (4× default 256 px per axis) for sharp eye/mouth edges.
pub const CLIPPING_MASK_BUFFER_SIZE: i32 = 1024;
/// Requested anisotropic filtering level (clamped to GPU MAX_TEXTURE_MAX_ANISOTROPY_EXT).
pub const TEXTURE_ANISOTROPY: f32 = 16.0;
/// Textures are uploaded premultiplied (`usePremultiply = true`).
pub const USE_PREMULTIPLY: bool = true;

/// The driver's per-frame `performance.now()` delta cap (seconds).
pub const PLAYER_DELTA_CAP: f64 = 0.1;
