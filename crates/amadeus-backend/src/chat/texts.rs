//! Verbatim strings of the V1 chat API (`spec-server-chat.md` §15). The frontend substring-matches
//! several of them (`handleApiError`), so they must stay byte-identical to V1.

/// Assistant reply sent instead of an answer when a FreeAPI request is rate limited.
pub const FREE_FALLBACK_RATE_LIMITED: &str =
    "[SAD] ごめんなさい、リクエストが多すぎるみたい……。少し休ませてからまた話しかけてね。";

/// Assistant reply sent instead of an answer when FreeAPI cannot reach any upstream.
pub const FREE_FALLBACK_COMM_ERROR: &str =
    "[SAD] ごめんなさい、ちょっと通信の調子が悪いみたい……。少し待ってからもう一度試してみてくれる？";

/// Reply text when an upstream answer could not be parsed (returned as normal text by V1).
pub const PARSE_ERROR: &str = "[Parse Error]";

/// 400 body error for unparseable payloads or payloads without a `messages` array.
pub const ERR_INVALID_PAYLOAD: &str = "Invalid request payload";

/// Default 429 message when the limiter gives no reason (unreachable in V1, kept for parity).
pub const ERR_TOO_MANY_REQUESTS_DEFAULT: &str = "Too many requests. Please wait a minute.";

/// Per-IP sliding window exceeded.
pub const ERR_RATE_LIMITED: &str = "リクエストが多すぎます。1分ほど待ってから再度お試しください。";

/// Global FreeAPI cap reached (`max` is interpolated; V1 used 65000).
pub fn global_cap_message(max: i64) -> String {
    format!("FreeAPIの全体リクエスト上限（{max}回）に達しました。サービスを停止しています。")
}

/// Missing API key (providers 0, 1, 2, 3, 6).
pub const ERR_NO_API_KEY: &str = "API Key が設定されていません。CONFIGから設定してください。";

/// `route.ts` Vertex B: the key looks like JSON but is not a service account.
pub const ERR_VERTEX_SA_JSON_INVALID: &str =
    "Vertex AI: サービスアカウントJSONキーの形式が不正です。";

/// `route.ts` Vertex B1: no project id for the service-account path.
pub const ERR_VERTEX_SA_PROJECT_REQUIRED: &str =
    "Vertex AI: サービスアカウントJSON利用時は Project ID が必要です。";

/// `route.ts` Vertex B2: no project id for the gcloud path.
pub const ERR_VERTEX_GCLOUD_PROJECT_REQUIRED: &str =
    "Vertex AI: gcloud認証利用時は Project ID が必要です。";

/// `route.ts` Vertex B2: neither a key nor a gcloud token.
pub const ERR_VERTEX_NO_CREDENTIALS_FOUND: &str =
    "Vertex AI: 認証情報が見つかりません。APIキーを設定するか、サーバーの gcloud auth login を実行してください。";

/// `vertex-helpers.ts` `getVertexAccessToken`: no usable credential at all.
pub const ERR_VERTEX_NO_CREDENTIALS: &str =
    "Vertex AI: 認証情報がありません。CONFIGからサービスアカウントJSONキーを設定してください。";

/// FreeAPI Vertex fallback without a server gcloud token.
pub const ERR_FREEAPI_NO_GCLOUD: &str =
    "FreeAPI: gcloud 認証情報がありません。サーバーの設定を確認してください。";

/// Vertex helpers / vertex-proxy: `vertexProject` is empty.
pub const ERR_VERTEX_PROJECT_MISSING: &str =
    "Vertex AI: Project ID が未設定です。Config_VertexProject を設定してください。";

/// vertex-proxy: the gcloud CLI did not return a token (contains a real LF).
pub const ERR_VERTEX_PROXY_NO_TOKEN: &str = "Vertex AI: アクセストークンの取得に失敗しました。\nサーバーに gcloud CLI がインストールされ、gcloud auth login 済みか確認してください。";

/// Desktop Vertex AI: the local gcloud CLI did not return an access token.
pub const ERR_VERTEX_DESKTOP_NO_TOKEN: &str = "Vertex AI: アクセストークンの取得に失敗しました。\ngcloud CLI がインストールされ、gcloud auth login 済みか確認してください。";

/// `/chat` default branch.
pub const ERR_UNKNOWN_PROVIDER: &str = "Unknown provider.";

/// `/chat` catch for non-`Error` throws (kept for parity; every Rust error has a message).
pub const ERR_UNKNOWN_SERVER: &str = "Unknown server error";

/// `/chat/stream` catch for non-`Error` throws.
pub const ERR_UNKNOWN_STREAMING: &str = "Unknown streaming error";

/// `streamOpenAILike` with an unexpected provider (unreachable in V1).
pub const ERR_UNSUPPORTED_OPENAI_LIKE: &str = "Unsupported provider for streamOpenAILike";

/// undici's message for every network / transport failure before the response headers.
pub const FETCH_FAILED: &str = "fetch failed";

/// undici's message when reading a response body fails mid-way.
pub const TERMINATED: &str = "terminated";

/// Error thrown by Node `crypto.sign` for keys it cannot decode.
pub const ERR_KEY_UNSUPPORTED: &str = "error:1E08010C:DECODER routines::unsupported";

/// Error thrown by Node `crypto.sign` for an empty key.
pub const ERR_KEY_EMPTY: &str = "No key provided to sign";
