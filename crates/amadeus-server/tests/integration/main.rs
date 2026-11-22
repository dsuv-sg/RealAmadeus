//! Integration tests of `amadeus-server` (one test binary): the full router driven with
//! `tower::ServiceExt::oneshot` against a temporary Trunk-like dist. Proxy targets (TTS / STT /
//! RAG servers, Ollama, Vertex) are a recording mock upstream on 127.0.0.1; nothing leaves the
//! machine.

mod aux_proxy;
mod common;
mod local_proxy;
mod routing;
mod sse;
