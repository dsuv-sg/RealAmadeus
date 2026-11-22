//! The browser half of V1 `lib/amadeus/services/vector-database.ts`: the HTTP calls (health and
//! embeddings) and the localStorage persistence of the store.
//!
//! TODO(chat engineer): `amadeus_core::vector` (entries, cosine/lexical search, top-K,
//! threshold, `getContextForPrompt`, the `RealAmadeus.VectorStore` JSON format) was still an
//! empty stub when this client was written. Build V1's `VectorDatabase` on top of it with:
//! * [`RagClient::request_embedding`] for `requestEmbedding` (`addEntry` / `search`),
//! * [`RagClient::load_store_json`] / [`RagClient::save_store_json`] for `load()` / `save()`,
//! * [`RagClient::check_server_health`] / [`RagClient::server_available`] for the health part.
//!
//! The TODO also covers the V1 `VectorDatabase` members this client lacks: the constructor's
//! `load()`, the `entryAdded` / `searchComplete` / `searchError` signals, and the setters with
//! their `*Changed` signals, which CONFIG "Apply" calls after writing the keys (CFG-003,
//! CFG-020). Each setter is a no-op when the value is unchanged:
//! * `enabled` / `serverUrl`: persist [`keys::RAG_ENABLED`] / [`keys::RAG_SERVER_URL`], emit,
//!   then [`RagClient::check_server_health`] (`enabled`: only when turned on);
//! * `topK`: `max(1, min(20, trunc(v)))` into [`keys::RAG_TOP_K`], then `topKChanged`;
//! * `threshold`: `max(0, min(1, v))` (unchanged within `Number.EPSILON`) into
//!   [`keys::RAG_THRESHOLD`], then `thresholdChanged`.
//!
//! After Apply, only an out-of-range top-K / threshold makes a setter act: it rewrites the key
//! with the clamped value and emits (CFG-020 "written twice when clamped").
//!
//! The settings (`Config_Experimental_RAG_*`) are read at call time like the other services.

use std::cell::Cell;
use std::future::Future;
use std::rc::{Rc, Weak};

use amadeus_core::protocol::RagEmbedBody;
use amadeus_core::settings::keys;
use amadeus_core::storage::KvStore;
use amadeus_core::types::Emitter;

use super::logic::embedding_from_reply;
use super::settings;
use crate::transport::transport;

/// HTTP/persistence client of V1 `VectorDatabase`.
pub struct RagClient {
    /// `serverAvailableChanged`.
    pub server_available_changed: Emitter<()>,
    server_available: Cell<bool>,
    weak_self: Weak<RagClient>,
}

impl RagClient {
    /// V1 constructor part: health check right away when RAG is enabled.
    pub(crate) fn new() -> Rc<Self> {
        let client = Rc::new_cyclic(|weak| Self {
            server_available_changed: Emitter::new(),
            server_available: Cell::new(false),
            weak_self: weak.clone(),
        });
        if client.enabled() {
            client.check_server_health();
        }
        client
    }

    /// V1 `enabled` (`Config_Experimental_RAG_Enabled === 1`).
    pub fn enabled(&self) -> bool {
        keys::RAG_ENABLED.get(settings())
    }

    /// V1 `serverUrl` (`Config_Experimental_RAG_ServerUrl`, default `http://localhost:5102`).
    pub fn server_url(&self) -> String {
        keys::RAG_SERVER_URL.get(settings())
    }

    /// V1 `serverAvailable`.
    pub fn server_available(&self) -> bool {
        self.server_available.get()
    }

    /// V1 `VectorDatabase.checkServerHealth()` (background; called at app start even when RAG
    /// is disabled): `POST app/api/rag/health`, `ok && Boolean(data?.available)`, emitting
    /// `server_available_changed` on change.
    pub fn check_server_health(&self) {
        let server_url = self.server_url();
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let available = transport()
                .rag_health(&server_url)
                .await
                .map(|r| r.available())
                .unwrap_or(false);
            let Some(this) = weak.upgrade() else { return };
            if this.server_available.get() != available {
                this.server_available.set(available);
                this.server_available_changed.emit(&());
            }
        });
    }

    /// V1 `requestEmbedding(text)`: `POST app/api/rag/embed` with `{serverUrl, text}` (the raw,
    /// untrimmed text; the server trims). Resolves to `[]` on any failure (silent, like V1),
    /// otherwise every item of `data.embedding` through JS `Number()`. The server URL is read
    /// now, when the request is created.
    pub fn request_embedding(&self, text: &str) -> impl Future<Output = Vec<f64>> + 'static {
        let body = RagEmbedBody {
            server_url: self.server_url(),
            text: text.to_owned(),
        };
        async move {
            match transport().rag_embed(&body).await {
                Ok(reply) => embedding_from_reply(&reply),
                Err(_) => Vec::new(),
            }
        }
    }

    /// V1 `load()`: the raw `RealAmadeus.VectorStore` JSON, if stored.
    pub fn load_store_json(&self) -> Option<String> {
        crate::storage::local_store()
            .get(keys::VECTOR_STORE_STORAGE_KEY)
            .filter(|raw| !raw.is_empty())
    }

    /// V1 `save()`: writes the `RealAmadeus.VectorStore` JSON (`{"version":1,"entries":[…]}`).
    pub fn save_store_json(&self, json: &str) {
        crate::storage::local_store().set(keys::VECTOR_STORE_STORAGE_KEY, json);
    }
}
