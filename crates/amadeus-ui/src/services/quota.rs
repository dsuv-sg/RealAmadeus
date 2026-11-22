//! V1 `fetchFreeApiQuota()` (`RealAmadeusApp.tsx`): the FreeAPI global quota shown in CONFIG
//! while provider 8 is selected (spec-app-part4 §3.11 / §7.12).

pub use super::logic::FreeApiQuota;
use crate::transport::transport;

/// `GET {base}/app/api/chat/quota` with `cache: "no-store"` (desktop: `chat_quota`, answered
/// by the hosted server). `None` when the request fails, the status is not OK, or the body is
/// not `{count, max, remaining}` with three numbers — V1 then hides the block.
///
/// Callers discard a result that arrives after their condition turned false (V1's `cancelled`
/// flag) and keep showing the previous quota until the new one resolves.
pub async fn fetch_free_api_quota() -> Option<FreeApiQuota> {
    let reply = transport().chat_quota().await.ok()?;
    FreeApiQuota::from_reply(&reply)
}
