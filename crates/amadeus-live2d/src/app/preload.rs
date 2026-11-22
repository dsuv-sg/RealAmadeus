//! Warm the asset caches with the default model (V1 `preloadDefaultAssets`, pipeline spec §6.2).

use wasm_bindgen::prelude::*;

use super::assets;
use super::define::{MODEL_DIR, PRELOAD_BINARIES, PRELOAD_IMAGE};
use super::url::ModelHome;

/// Prefetch model3.json, moc3, physics3.json (bytes) and decode the texture, all in parallel,
/// under `resources_base` (e.g. `"resources/"`, resolved against `document.baseURI`). Errors are
/// swallowed like V1 (every task had `.catch(() => {})`), so this always returns `Ok(())`; a failed
/// entry is evicted and retried by the player. The file list is hard-coded (no model3.json round
/// trip); the 13 shader fetches of V1 are gone because the shaders are embedded (D4). Works before
/// the Core is ready.
pub async fn preload_default_assets(resources_base: &str) -> Result<(), JsValue> {
    let home = ModelHome::new(resources_base, MODEL_DIR);
    // Issue every request first (they start immediately), then wait for all of them.
    let binaries: Vec<_> = PRELOAD_BINARIES
        .iter()
        .map(|file| assets::warm_bytes(&home.file_url(file)))
        .collect();
    let image = assets::warm_image(&home.file_url(PRELOAD_IMAGE));
    for task in binaries {
        let _ = task.await;
    }
    let _ = image.await;
    Ok(())
}
