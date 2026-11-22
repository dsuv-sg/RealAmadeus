//! CharacterMemory (1–13) Rust engine:
//!
//! 1. **Decay & Relevance Repair (`decay.py`)**: creation-age exponential decay with emotion-extended
//!    half-life, bounded 10 % familiarity exposure boost (`recall / (recall + 10)`), and 110 %
//!    query-relevance repair (`familiar + relevance * 1.10 * (intrinsic - familiar)`).
//! 2. **BM25 + RRF Hybrid Contextual Search (`rag/hybrid.py`)**: history-weighted multi-query
//!    retrieval (`retrieval_recency_decay ** i` over the recent user turns) using CJK character
//!    bigrams + word tokens, BM25 scoring, and Reciprocal Rank Fusion (RRF).
//! 3. **Emotional Impact & Mood Congruence (`episodic.py` / `emotion_vectors.py`)**: 5-axis sparse
//!    emotion vectors (`joy`, `sadness`, `anger`, `anxiety`, `affection`) mapped from Kurisu's 11
//!    `Emotion` tags, L2 emotional impact magnitude, and cosine similarity mood congruence.
//! 4. **Enhanced User Facts (`user_facts.py`)**: dynamic salience + relevance selection.
//! 5. **Enhanced Episodic Memory (`episodic.py`)**: emotion-weighted decay + mood-congruent recall.
//! 6. **User Directives (`user_directives.py`)**: standing instructions / rules ("call me X",
//!    "don't use formal speech", promises) separated into a dedicated sticky section with keyword
//!    boosts so they are never crowded out by ordinary facts.
//! 7. **Knowledge Graph Spreading Activation (`knowledge_graph/`)**: graph over `Self`, `Person`,
//!    `Fact`, `Episode`, and `Entity`/`Tag` nodes; query/emotion activation spreads along co-occurrence
//!    edges to surface indirectly linked memories.
//! 8. **Character Info & Dialogue Style RAG (`character_info` / `dialogue_style`)**: static
//!    Steins;Gate / Amadeus Kurisu lore chunks and few-shot dialogue style examples dynamically
//!    retrieved via BM25 when the conversation touches relevant topics.
//! 9. **Zero-Network Temporal Expression Resolution (`temporal/`)**: resolves Japanese and English
//!    relative time expressions ("昨日"/"yesterday", "一昨日"/"day before yesterday", "先週"/"last week",
//!    "今日"/"today", "明日"/"tomorrow", "来週"/"next week", `N日前`/`N days ago`, explicit `MM/DD` or
//!    `YYYY-MM-DD`) into `[start_day, end_day]` intervals to boost matching episodes and calendar
//!    items.
//! 10. **Sticky Slot Partitioning & Score Normalization (`reranking.py` / `structured.py`)**:
//!     normalizes positive scores per memory category, partitions sticky high-strength facts from
//!     relevance-driven slots, and enforces section budgets.
//! 11. **Emotion & Affinity Relationship Tracking (`emotion.py`)**: derives a rolling 5-axis mood
//!     vector and bond/affinity level from interaction count, fact strength, and recent emotion
//!     history.
//! 12. **User Summary (`user_summary.py`)**: synthesizes a compact rolling user profile summary
//!     from top facts, directives, and interaction history.
//! 13. **Calendar & Prospective Event Memory (`calendar.py`)**: detects scheduled / dated /
//!     anniversary / future or past dated events in facts and episodes and surfaces upcoming or
//!     temporally referenced events.

use std::collections::{BTreeMap, BTreeSet};

use crate::jscompat::js_trim;
use crate::protocol::{ChatMessage, ChatRole};

use super::time::{parse_iso_date_day, LocalDateTime};
use super::{certainty_label, compare_episodes, compare_facts, stable_sorted};
use super::{
    Collator, EpisodicMemory, MemoryPayload, UserFact, PROMPT_EPISODE_LIMIT, PROMPT_FACT_LIMIT,
};

// ---------------------------------------------------------------------------------------------
// Constants (`decay.py`, `reranking.py`, `agent.py`)
// ---------------------------------------------------------------------------------------------

/// Maximum familiarity boost from prior exposures (`decay.py`: `MAX_EXPOSURE_BOOST = 0.10`).
pub const MAX_EXPOSURE_BOOST: f64 = 0.10;
/// Half-saturation constant for exposure (`decay.py`: `EXPOSURE_HALF_SATURATION = 10.0`).
pub const EXPOSURE_HALF_SATURATION: f64 = 10.0;
/// Maximum decay loss restored by query relevance (`decay.py`: `MAX_RELEVANCE_REPAIR = 1.10`).
pub const MAX_RELEVANCE_REPAIR: f64 = 1.10;
/// Default half-life of a memory in days (30 days).
pub const DEFAULT_HALF_LIFE_DAYS: f64 = 30.0;
/// Number of recent user messages included in history-aware retrieval (`retrieval_history_window`).
pub const RETRIEVAL_HISTORY_WINDOW: usize = 3;
/// Per-turn recency decay for earlier user messages (`retrieval_recency_decay`).
pub const RETRIEVAL_RECENCY_DECAY: f64 = 0.65;
/// RRF smoothing constant (`k = 60` in standard Reciprocal Rank Fusion).
pub const RRF_K: f64 = 60.0;
/// BM25 `k1` term-frequency saturation parameter.
pub const BM25_K1: f64 = 1.5;
/// BM25 `b` document-length normalization parameter.
pub const BM25_B: f64 = 0.75;
/// Maximum sticky slots reserved in fact recall so relevance-driven facts are never crowded out.
pub const STICKY_FACT_SLOTS: usize = 2;
/// Maximum directives shown in the prompt.
pub const PROMPT_DIRECTIVE_LIMIT: usize = 4;
/// Maximum calendar / schedule entries shown in the prompt.
pub const PROMPT_CALENDAR_LIMIT: usize = 3;
/// Maximum character relationship chunks shown in the prompt.
pub const PROMPT_RELATIONSHIP_LIMIT: usize = 3;
/// Maximum character lore chunks shown in the prompt.
pub const PROMPT_LORE_LIMIT: usize = 3;
/// Maximum dialogue style examples shown in the prompt.
pub const PROMPT_STYLE_LIMIT: usize = 2;

// ---------------------------------------------------------------------------------------------
// #1. Decay & Relevance Repair (`decay.py`)
// ---------------------------------------------------------------------------------------------

/// `count / (count + half_saturation)` in `[0, 1]`.
pub fn exposure_saturation(recall_count: u32, half_saturation: f64) -> f64 {
    let count = f64::from(recall_count).max(0.0);
    let half = half_saturation.max(1e-9);
    count / (count + half)
}

/// Query-independent salience before time decay (`base * (1 + clamp(emotion_impact, 0, 1))`).
pub fn intrinsic_score(base_importance: f64, emotion_impact: Option<f64>) -> f64 {
    let base = base_importance.max(0.0);
    match emotion_impact {
        Some(impact) if impact.is_finite() => base * (1.0 + impact.clamp(0.0, 1.0)),
        _ => base,
    }
}

/// Non-negative query-independent familiar salience (`decay.py::decay_score`).
pub fn decay_score(
    base_importance: f64,
    recall_count: u32,
    age_days: f64,
    half_life_days: f64,
    emotion_impact: Option<f64>,
) -> f64 {
    let impact = emotion_impact
        .filter(|v| v.is_finite())
        .map_or(0.0, |v| v.clamp(0.0, 1.0));
    let decay_resistance = if emotion_impact.is_some() {
        1.0 + impact
    } else {
        1.0
    };
    let adjusted_half_life = half_life_days * decay_resistance;
    let time_factor = if adjusted_half_life <= 0.0 {
        1.0
    } else {
        (-age_days.max(0.0) / adjusted_half_life).exp()
    };
    let intrinsic = intrinsic_score(base_importance, emotion_impact);
    let aged = intrinsic * time_factor;
    let familiarity = intrinsic
        * MAX_EXPOSURE_BOOST
        * exposure_saturation(recall_count, EXPOSURE_HALF_SATURATION);
    intrinsic.min(aged + familiarity)
}

/// Restores up to `MAX_RELEVANCE_REPAIR` (110 %) of decay loss when query relevance is high.
pub fn relevance_repaired_score(intrinsic: f64, familiar: f64, relevance: f64) -> f64 {
    let intrinsic = intrinsic.max(0.0);
    let familiar = familiar.max(0.0);
    let rel = relevance.clamp(0.0, 1.0);
    let lost = (intrinsic - familiar).max(0.0);
    familiar + rel * MAX_RELEVANCE_REPAIR * lost
}

// ---------------------------------------------------------------------------------------------
// #3 & #11. Emotion Vectors, Emotional Impact, Mood Congruence & Affinity (`emotion_vectors.py`, `emotion.py`)
// ---------------------------------------------------------------------------------------------

/// 5-axis sparse emotion vector (`[joy, sadness, anger, anxiety, affection]` in `[0, 1]`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EmotionVector {
    pub joy: f64,
    pub sadness: f64,
    pub anger: f64,
    pub anxiety: f64,
    pub affection: f64,
}

impl EmotionVector {
    pub const fn new(joy: f64, sadness: f64, anger: f64, anxiety: f64, affection: f64) -> Self {
        Self {
            joy,
            sadness,
            anger,
            anxiety,
            affection,
        }
    }

    /// Baseline mood of Amadeus Kurisu (`neutral/calm` with mild intellectual curiosity).
    pub const fn baseline() -> Self {
        Self::new(0.25, 0.10, 0.05, 0.05, 0.25)
    }

    /// Maps one of Kurisu's 11 emotion tags (or `SLEEPING`) to a 5-axis emotion vector.
    pub fn from_tag(tag: &str) -> Self {
        match tag.trim().to_ascii_uppercase().as_str() {
            "SMILE" => Self::new(0.85, 0.0, 0.0, 0.0, 0.70),
            "WINK" => Self::new(0.80, 0.0, 0.0, 0.0, 0.75),
            "BLUSH" => Self::new(0.45, 0.0, 0.15, 0.35, 0.95),
            "SMUG" => Self::new(0.75, 0.0, 0.0, 0.0, 0.50),
            "ANGRY" => Self::new(0.0, 0.05, 0.90, 0.20, 0.15),
            "DISGUST" => Self::new(0.0, 0.10, 0.75, 0.25, 0.05),
            "SAD" => Self::new(0.0, 0.90, 0.05, 0.45, 0.40),
            "PANIC" => Self::new(0.0, 0.25, 0.20, 0.95, 0.35),
            "SURPRISED" => Self::new(0.35, 0.05, 0.05, 0.70, 0.35),
            "THINKING" => Self::new(0.30, 0.05, 0.0, 0.15, 0.35),
            "SLEEPING" => Self::new(0.20, 0.05, 0.0, 0.0, 0.30),
            _ => Self::baseline(),
        }
    }

    /// Infers an emotion vector from episode/fact text and tags when no explicit tag was stored.
    pub fn infer_from_text_and_tags(
        text: &str,
        tags: &[String],
        fallback_tag: Option<&str>,
    ) -> Self {
        let mut vec = fallback_tag.map_or_else(Self::baseline, Self::from_tag);
        let combined = format!("{} {}", text.to_lowercase(), tags.join(" ").to_lowercase());
        let boost = |v: &mut f64, delta: f64| *v = (*v + delta).clamp(0.0, 1.0);

        if [
            "嬉しい",
            "楽し",
            "笑",
            "喜",
            "成功",
            "感謝",
            "ありがとう",
            "happy",
            "fun",
            "smile",
            "thank",
        ]
        .iter()
        .any(|k| combined.contains(k))
        {
            boost(&mut vec.joy, 0.45);
            boost(&mut vec.affection, 0.35);
        }
        if [
            "照れ",
            "恥ずかし",
            "好き",
            "大切",
            "約束",
            "blush",
            "love",
            "promise",
            "dear",
        ]
        .iter()
        .any(|k| combined.contains(k))
        {
            boost(&mut vec.affection, 0.55);
            boost(&mut vec.joy, 0.25);
        }
        if [
            "怒",
            "ケンカ",
            "喧嘩",
            "変態",
            "hentai",
            "angry",
            "mad",
            "argue",
        ]
        .iter()
        .any(|k| combined.contains(k))
        {
            boost(&mut vec.anger, 0.60);
        }
        if [
            "悲し",
            "寂し",
            "泣",
            "失敗",
            "つらい",
            "辛い",
            "sad",
            "lonely",
            "cry",
            "regret",
        ]
        .iter()
        .any(|k| combined.contains(k))
        {
            boost(&mut vec.sadness, 0.65);
        }
        if [
            "驚", "焦", "不安", "危険", "大変", "緊急", "panic", "worry", "anxious", "surprise",
        ]
        .iter()
        .any(|k| combined.contains(k))
        {
            boost(&mut vec.anxiety, 0.60);
        }
        vec
    }

    fn as_array(self) -> [f64; 5] {
        [
            self.joy,
            self.sadness,
            self.anger,
            self.anxiety,
            self.affection,
        ]
    }

    /// Clamped L2 magnitude in `[0, 1]` (`emotion_vectors.py::emotional_impact`).
    pub fn emotional_impact(self) -> f64 {
        let sum_sq: f64 = self.as_array().iter().map(|v| v * v).sum();
        sum_sq.sqrt().clamp(0.0, 1.0)
    }

    /// Cosine similarity in `[0, 1]` (`emotion_vectors.py::emotion_similarity`).
    pub fn similarity(self, other: Self) -> f64 {
        let a = self.as_array();
        let b = other.as_array();
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm_a <= 1e-9 || norm_b <= 1e-9 {
            return 0.0;
        }
        (dot / (norm_a * norm_b)).clamp(0.0, 1.0)
    }
}

/// Computes the character's current rolling mood vector from `recentEmotions`.
pub fn current_mood_vector(recent_emotions: &[String]) -> EmotionVector {
    if recent_emotions.is_empty() {
        return EmotionVector::baseline();
    }
    let mut sum = [0.0_f64; 5];
    let mut weight_total = 0.0;
    for (idx, tag) in recent_emotions.iter().rev().enumerate() {
        let w = 0.7_f64.powi(idx as i32);
        let v = EmotionVector::from_tag(tag).as_array();
        for (s, x) in sum.iter_mut().zip(v.iter()) {
            *s += w * x;
        }
        weight_total += w;
    }
    if weight_total <= 1e-9 {
        return EmotionVector::baseline();
    }
    EmotionVector::new(
        (sum[0] / weight_total).clamp(0.0, 1.0),
        (sum[1] / weight_total).clamp(0.0, 1.0),
        (sum[2] / weight_total).clamp(0.0, 1.0),
        (sum[3] / weight_total).clamp(0.0, 1.0),
        (sum[4] / weight_total).clamp(0.0, 1.0),
    )
}

/// Computes user bond / affinity status (`emotion.py` relationship tracking) from interaction count,
/// remembered facts, and recent emotions.
pub fn relationship_affinity_label(
    total_interactions: i64,
    fact_count: usize,
    episode_count: usize,
    mood: EmotionVector,
) -> (&'static str, i64) {
    let base = (total_interactions.max(0) * 2)
        + (fact_count as i64 * 4)
        + (episode_count as i64 * 5)
        + ((mood.affection * 20.0).round() as i64);
    let score = base.clamp(0, 100);
    let label = match score {
        80..=100 => "深い信頼関係（大切なパートナー・素直になれないが特別に意識している）",
        50..=79 => "親密な仲間（気の置けない話し相手・ツンデレしつつ信頼している）",
        25..=49 => "良好な関係（知的な対話を楽しむ相手）",
        _ => "初期段階（様子見・知的好奇心を持って接している）",
    };
    (label, score)
}

// ---------------------------------------------------------------------------------------------
// #2. Multilingual Tokenizer (Words + CJK Bigrams) & BM25 + RRF Hybrid Search (`rag/hybrid.py`)
// ---------------------------------------------------------------------------------------------

fn is_cjk_or_kana(c: char) -> bool {
    matches!(
        c as u32,
        0x3040..=0x309F // Hiragana
        | 0x30A0..=0x30FF // Katakana
        | 0x3400..=0x4DBF // CJK Ext A
        | 0x4E00..=0x9FFF // CJK Unified
        | 0xAC00..=0xD7AF // Hangul
        | 0xF900..=0xFAFF // CJK Compatibility
    )
}

/// Tokenizes text into lowercased Latin/Cyrillic words AND overlapping CJK/Kana character
/// bigrams + unigrams so Japanese, Chinese, Korean, and European languages all work without
/// external segmentation dictionaries.
pub fn tokenize_multilingual(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens = Vec::new();
    let mut latin_buf = String::new();
    let mut cjk_buf: Vec<char> = Vec::new();

    let flush_latin = |buf: &mut String, out: &mut Vec<String>| {
        if !buf.is_empty() {
            if buf.chars().count() >= 2 || buf.chars().all(|c| c.is_ascii_digit()) {
                out.push(std::mem::take(buf));
            } else {
                buf.clear();
            }
        }
    };
    let flush_cjk = |buf: &mut Vec<char>, out: &mut Vec<String>| {
        if buf.is_empty() {
            return;
        }
        if buf.len() == 1 {
            out.push(buf[0].to_string());
        } else {
            for win in buf.windows(2) {
                let mut s = String::with_capacity(8);
                s.push(win[0]);
                s.push(win[1]);
                out.push(s);
            }
            // Also push Kanji unigrams so single-kanji concepts ("猫", "夜", "酒") match.
            for &ch in buf.iter() {
                if (0x4E00..=0x9FFF).contains(&(ch as u32)) {
                    out.push(ch.to_string());
                }
            }
        }
        buf.clear();
    };

    for ch in lower.chars() {
        if is_cjk_or_kana(ch) {
            flush_latin(&mut latin_buf, &mut tokens);
            cjk_buf.push(ch);
        } else if ch.is_alphanumeric() || ch == '_' || ch == '@' {
            flush_cjk(&mut cjk_buf, &mut tokens);
            latin_buf.push(ch);
        } else {
            flush_latin(&mut latin_buf, &mut tokens);
            flush_cjk(&mut cjk_buf, &mut tokens);
        }
    }
    flush_latin(&mut latin_buf, &mut tokens);
    flush_cjk(&mut cjk_buf, &mut tokens);
    tokens
}

/// Computes BM25 + lexical overlap scores in `[0, 1]` for a slice of documents against a
/// history-weighted multi-query (`[(query_text, weight)]`).
pub fn score_documents_bm25_rrf(docs: &[String], queries: &[(String, f64)]) -> Vec<f64> {
    if docs.is_empty() || queries.is_empty() {
        return vec![0.0; docs.len()];
    }
    let tokenized_docs: Vec<Vec<String>> = docs.iter().map(|d| tokenize_multilingual(d)).collect();
    let total_len: usize = tokenized_docs.iter().map(Vec::len).sum();
    let avg_dl = (total_len as f64 / docs.len() as f64).max(1.0);
    let n_docs = docs.len() as f64;

    // Document frequency table.
    let mut df: BTreeMap<&str, usize> = BTreeMap::new();
    for doc_tokens in &tokenized_docs {
        let unique: BTreeSet<&str> = doc_tokens.iter().map(String::as_str).collect();
        for tok in unique {
            *df.entry(tok).or_insert(0) += 1;
        }
    }

    let mut rrf_scores = vec![0.0_f64; docs.len()];
    let mut raw_scores = vec![0.0_f64; docs.len()];

    for (query_text, weight) in queries {
        let q_tokens = tokenize_multilingual(query_text);
        if q_tokens.is_empty() || *weight <= 0.0 {
            continue;
        }
        let mut per_doc = vec![0.0_f64; docs.len()];
        for (doc_idx, doc_tokens) in tokenized_docs.iter().enumerate() {
            if doc_tokens.is_empty() {
                continue;
            }
            let dl = doc_tokens.len() as f64;
            let mut score = 0.0;
            for q_tok in &q_tokens {
                let tf = doc_tokens.iter().filter(|t| *t == q_tok).count() as f64;
                if tf <= 0.0 {
                    continue;
                }
                let doc_freq = *df.get(q_tok.as_str()).unwrap_or(&0) as f64;
                let idf = ((n_docs - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();
                let denom = tf + BM25_K1 * (1.0 - BM25_B + BM25_B * (dl / avg_dl));
                score += idf * (tf * (BM25_K1 + 1.0)) / denom;
            }
            per_doc[doc_idx] = score;
            raw_scores[doc_idx] += score * weight;
        }

        // Rank documents with positive score for RRF fusion.
        let mut ranked: Vec<(usize, f64)> = per_doc
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, s)| *s > 0.0)
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (rank, (doc_idx, _)) in ranked.into_iter().enumerate() {
            rrf_scores[doc_idx] += weight / (RRF_K + (rank + 1) as f64);
        }
    }

    // Combine normalized BM25 and RRF into `[0, 1]`.
    let max_raw = raw_scores.iter().copied().fold(0.0_f64, f64::max);
    let max_rrf = rrf_scores.iter().copied().fold(0.0_f64, f64::max);
    docs.iter()
        .enumerate()
        .map(|(i, _)| {
            let norm_raw = if max_raw > 1e-9 {
                raw_scores[i] / max_raw
            } else {
                0.0
            };
            let norm_rrf = if max_rrf > 1e-9 {
                rrf_scores[i] / max_rrf
            } else {
                0.0
            };
            (0.65 * norm_raw + 0.35 * norm_rrf).clamp(0.0, 1.0)
        })
        .collect()
}

/// Extracts up to [`RETRIEVAL_HISTORY_WINDOW`] recent user messages from `history`, weighted by
/// `RETRIEVAL_RECENCY_DECAY ** i` (`agent.py` step 2).
pub fn build_weighted_queries(history: &[ChatMessage]) -> Vec<(String, f64)> {
    let mut queries = Vec::new();
    for msg in history
        .iter()
        .rev()
        .filter(|m| m.role == ChatRole::User && !js_trim(&m.content).is_empty())
        .take(RETRIEVAL_HISTORY_WINDOW)
    {
        let i = queries.len() as i32;
        let weight = RETRIEVAL_RECENCY_DECAY.powi(i);
        queries.push((msg.content.clone(), weight));
    }
    queries
}

// ---------------------------------------------------------------------------------------------
// #6. User Directives (`user_directives.py`)
// ---------------------------------------------------------------------------------------------

/// Returns true if a fact represents a standing user directive / instruction / promise / naming rule.
pub fn is_user_directive(content: &str) -> bool {
    let lower = content.to_lowercase();
    const DIRECTIVE_MARKERS: [&str; 28] = [
        "呼んで",
        "呼び方",
        "呼ばないで",
        "敬語",
        "タメ口",
        "口調",
        "約束",
        "禁止",
        "しないで",
        "やめて",
        "忘れないで",
        "覚えておいて",
        "ルール",
        "設定して",
        "常に",
        "必ず",
        "call me",
        "don't call",
        "do not call",
        "never ",
        "always ",
        "promise",
        "remember to",
        "rule:",
        "speak in",
        "tone",
        "directive",
        "ニックネーム",
    ];
    DIRECTIVE_MARKERS.iter().any(|m| lower.contains(m))
}

// ---------------------------------------------------------------------------------------------
// #7. Knowledge Graph Spreading Activation (`knowledge_graph/`)
// ---------------------------------------------------------------------------------------------

/// Propagates query relevance across facts and episodes that share tags or high-salience entity
/// tokens (`knowledge_graph/numeric_activation.py` 1-hop spreading activation).
pub fn apply_knowledge_graph_activation(
    fact_texts: &[String],
    fact_relevances: &mut [f64],
    episodes: &[&EpisodicMemory],
    episode_relevances: &mut [f64],
) {
    // Collect activated entity/tag tokens from items that already matched the query.
    let mut entity_activation: BTreeMap<String, f64> = BTreeMap::new();

    for (ep, &rel) in episodes.iter().zip(episode_relevances.iter()) {
        if rel <= 0.05 {
            continue;
        }
        for tag in &ep.tags {
            let key = tag.to_lowercase();
            if key != "要約" && !key.is_empty() {
                let entry = entity_activation.entry(key).or_insert(0.0);
                *entry = entry.max(rel);
            }
        }
        for tok in tokenize_multilingual(&ep.summary) {
            if tok.chars().count() >= 2 {
                let entry = entity_activation.entry(tok).or_insert(0.0);
                *entry = entry.max(rel * 0.7);
            }
        }
    }

    for (text, &rel) in fact_texts.iter().zip(fact_relevances.iter()) {
        if rel <= 0.05 {
            continue;
        }
        for tok in tokenize_multilingual(text) {
            if tok.chars().count() >= 2 {
                let entry = entity_activation.entry(tok).or_insert(0.0);
                *entry = entry.max(rel * 0.7);
            }
        }
    }

    if entity_activation.is_empty() {
        return;
    }

    // Spread activation (damping factor 0.35) to connected facts and episodes.
    for (text, rel) in fact_texts.iter().zip(fact_relevances.iter_mut()) {
        let mut spread = 0.0_f64;
        for tok in tokenize_multilingual(text) {
            if let Some(&act) = entity_activation.get(&tok) {
                spread = spread.max(act * 0.35);
            }
        }
        *rel = (*rel + spread).clamp(0.0, 1.0);
    }

    for (ep, rel) in episodes.iter().zip(episode_relevances.iter_mut()) {
        let mut spread = 0.0_f64;
        for tag in &ep.tags {
            if let Some(&act) = entity_activation.get(&tag.to_lowercase()) {
                spread = spread.max(act * 0.45);
            }
        }
        for tok in tokenize_multilingual(&ep.summary) {
            if let Some(&act) = entity_activation.get(&tok) {
                spread = spread.max(act * 0.30);
            }
        }
        *rel = (*rel + spread).clamp(0.0, 1.0);
    }
}

// ---------------------------------------------------------------------------------------------
// #8. Character Info & Dialogue Style RAG (`character_info`, `dialogue_style`)
// ---------------------------------------------------------------------------------------------

pub use super::kurisu_profile::{
    CharacterChunk, KURISU_CHARACTER_INFO, KURISU_CORE_PERSONA_ANCHOR, KURISU_DIALOGUE_STYLE,
    KURISU_RELATIONSHIPS,
};

fn is_substantive_token(token: &str) -> bool {
    token
        .chars()
        .any(|c| !('\u{3040}'..='\u{309F}').contains(&c))
}

/// Retrieves relevant relationship, lore, or dialogue-style chunks using weighted keyword
/// specificity + BM25/RRF lexical matching with a meaningful-token overlap guard.
pub fn recall_character_chunks(
    chunks: &[CharacterChunk],
    queries: &[(String, f64)],
    limit: usize,
) -> Vec<&'static str> {
    if chunks.is_empty() || queries.is_empty() || limit == 0 {
        return Vec::new();
    }

    let docs: Vec<String> = chunks.iter().map(|c| c.text.to_owned()).collect();
    let bm25_scores = score_documents_bm25_rrf(&docs, queries);
    let doc_token_sets: Vec<Vec<String>> = docs.iter().map(|d| tokenize_multilingual(d)).collect();
    let query_data: Vec<(String, f64, BTreeSet<String>)> = queries
        .iter()
        .map(|(q, w)| {
            let tokens = tokenize_multilingual(q)
                .into_iter()
                .filter(|t| is_substantive_token(t))
                .collect();
            (q.to_lowercase(), *w, tokens)
        })
        .collect();

    let mut scored: Vec<(usize, f64)> = chunks
        .iter()
        .enumerate()
        .filter_map(|(idx, chunk)| {
            let mut kw_score = 0.0;
            for (q_idx, (q_lower, weight, _)) in query_data.iter().enumerate() {
                if *weight <= 0.0 {
                    continue;
                }
                let recency_bonus = if q_idx == 0 { 1.35 } else { 1.0 };
                for &kw in chunk.keywords {
                    if q_lower.contains(kw) {
                        let specificity = 1.0 + (kw.chars().count().min(10) as f64) * 0.08;
                        kw_score += weight * recency_bonus * specificity;
                    }
                }
            }

            let doc_set: BTreeSet<&str> = doc_token_sets[idx].iter().map(String::as_str).collect();
            let substantive_overlap = query_data
                .iter()
                .flat_map(|(_, _, q_tokens)| q_tokens.iter())
                .filter(|tok| doc_set.contains(tok.as_str()))
                .collect::<BTreeSet<_>>()
                .len();

            let bm25 = bm25_scores.get(idx).copied().unwrap_or(0.0);
            let total = if kw_score > 0.0 {
                kw_score + 0.85 * bm25
            } else if substantive_overlap >= 2 && bm25 > 0.0 {
                0.85 * bm25
            } else {
                0.0
            };

            (total > 0.0).then_some((idx, total))
        })
        .collect();

    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(idx, _)| chunks[idx].text)
        .collect()
}

// ---------------------------------------------------------------------------------------------
// #9 & #13. Temporal Expression Resolution & Calendar / Prospective Memory (`temporal/`, `calendar.py`)
// ---------------------------------------------------------------------------------------------

/// A resolved `[start_day, end_day]` interval (days since 1970-01-01) referenced by the user's
/// message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporalRange {
    pub start_day: i64,
    pub end_day: i64,
    pub label: &'static str,
}

impl TemporalRange {
    pub fn contains_day(self, day: i64) -> bool {
        day >= self.start_day && day <= self.end_day
    }
}

/// Resolves relative and explicit temporal expressions in `query` relative to `now`
/// (`temporal/` zero-network multilingual resolver).
pub fn resolve_temporal_expressions(query: &str, now: &LocalDateTime) -> Vec<TemporalRange> {
    let lower = query.to_lowercase();
    let today = now.day_number();
    let mut ranges = Vec::new();

    if lower.contains("一昨日")
        || lower.contains("おととい")
        || lower.contains("day before yesterday")
    {
        ranges.push(TemporalRange {
            start_day: today - 2,
            end_day: today - 2,
            label: "一昨日",
        });
    }
    if lower.contains("昨日") || lower.contains("きのう") || lower.contains("yesterday") {
        ranges.push(TemporalRange {
            start_day: today - 1,
            end_day: today - 1,
            label: "昨日",
        });
    }
    if lower.contains("今日") || lower.contains("きょう") || lower.contains("today") {
        ranges.push(TemporalRange {
            start_day: today,
            end_day: today,
            label: "今日",
        });
    }
    if lower.contains("明日") || lower.contains("あした") || lower.contains("tomorrow") {
        ranges.push(TemporalRange {
            start_day: today + 1,
            end_day: today + 1,
            label: "明日",
        });
    }
    if lower.contains("明後日")
        || lower.contains("あさって")
        || lower.contains("day after tomorrow")
    {
        ranges.push(TemporalRange {
            start_day: today + 2,
            end_day: today + 2,
            label: "明後日",
        });
    }
    if lower.contains("先週")
        || lower.contains("last week")
        || lower.contains("この前")
        || lower.contains("先日")
    {
        ranges.push(TemporalRange {
            start_day: today - 10,
            end_day: today - 1,
            label: "先日・先週",
        });
    }
    if lower.contains("来週") || lower.contains("next week") {
        ranges.push(TemporalRange {
            start_day: today + 1,
            end_day: today + 10,
            label: "来週",
        });
    }
    if let Some(day) = parse_iso_date_day(query) {
        ranges.push(TemporalRange {
            start_day: day,
            end_day: day,
            label: "指定日",
        });
    }
    ranges
}

/// Returns true if a fact or episode represents a calendar / schedule / anniversary entry
/// (`calendar.py`).
pub fn is_calendar_entry(text: &str, tags: &[String]) -> bool {
    let lower = text.to_lowercase();
    const CALENDAR_MARKERS: [&str; 24] = [
        "予定",
        "スケジュール",
        "誕生日",
        "記念日",
        "締切",
        "締め切り",
        "試験",
        "テスト",
        "会議",
        "旅行",
        "イベント",
        "約束の日",
        "明日",
        "来週",
        "来月",
        "月曜",
        "火曜",
        "水曜",
        "木曜",
        "金曜",
        "土曜",
        "日曜",
        "schedule",
        "birthday",
    ];
    if CALENDAR_MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    tags.iter().any(|t| {
        let tl = t.to_lowercase();
        matches!(
            tl.as_str(),
            "予定"
                | "カレンダー"
                | "誕生日"
                | "記念日"
                | "イベント"
                | "schedule"
                | "calendar"
                | "event"
        )
    })
}

// ---------------------------------------------------------------------------------------------
// #10 & #12. Sticky Partitioning, Reranking, User Summary & Full Contextual Prompt Assembly
// ---------------------------------------------------------------------------------------------

/// Builds the full memory context for the LLM system prompt.
///
/// * When `history` has no user message yet (`initial_history` / `memory_context()`), it produces
///   the exact V1 output so all existing tests and initial states stay 100 % bit-identical.
/// * When `history` contains active user conversation turns (`update_system_prompt`), it activates
///   CharacterMemory features 1–13:
///   - #1 Decay + Relevance Repair (`decay.py`)
///   - #2 History-weighted BM25 + RRF search (`rag/hybrid.py`)
///   - #3 & #11 Emotion Vector impact, Mood Congruence & Relationship Affinity (`emotion_vectors.py`, `emotion.py`)
///   - #4 & #5 Dynamic Fact & Episode recall (`user_facts.py`, `episodic.py`)
///   - #6 User Directives separation (`user_directives.py`)
///   - #7 Knowledge Graph Spreading Activation (`knowledge_graph/`)
///   - #8 Character Info & Dialogue Style RAG (`character_info`, `dialogue_style`)
///   - #9 Temporal Expression Resolution (`temporal/`)
///   - #10 Sticky slot partitioning & score normalization (`reranking.py`)
///   - #12 Rolling User Summary (`user_summary.py`)
///   - #13 Calendar & Prospective Event memory (`calendar.py`)
pub fn build_contextual_memory_prompt(
    memory: &MemoryPayload,
    history: &[ChatMessage],
    now: &LocalDateTime,
    collator: &dyn Collator,
    base_context: &str,
) -> String {
    let queries = build_weighted_queries(history);
    if queries.is_empty() {
        return base_context.to_owned();
    }

    let today_day = now.day_number();
    let recent_emotions = memory.recent_emotions.as_deref().unwrap_or(&[]);
    let current_mood = current_mood_vector(recent_emotions);
    let latest_query = &queries[0].0;
    let temporal_ranges = resolve_temporal_expressions(latest_query, now);

    let all_facts: Vec<&UserFact> = memory.user_facts.iter().flatten().collect();
    let all_episodes: Vec<&EpisodicMemory> = memory.episodic_memory.iter().flatten().collect();

    // --- #2 BM25 + RRF relevance over facts and episodes ---
    let fact_docs: Vec<String> = all_facts.iter().map(|f| f.content.clone()).collect();
    let mut fact_relevances = score_documents_bm25_rrf(&fact_docs, &queries);

    let episode_docs: Vec<String> = all_episodes
        .iter()
        .map(|ep| {
            if ep.tags.is_empty() {
                ep.summary.clone()
            } else {
                format!("{} {}", ep.summary, ep.tags.join(" "))
            }
        })
        .collect();
    let mut episode_relevances = score_documents_bm25_rrf(&episode_docs, &queries);

    // --- #7 Knowledge Graph 1-hop Spreading Activation ---
    apply_knowledge_graph_activation(
        &fact_docs,
        &mut fact_relevances,
        &all_episodes,
        &mut episode_relevances,
    );

    // --- #9 Temporal expression boost on episodes ---
    if !temporal_ranges.is_empty() {
        for (ep, rel) in all_episodes.iter().zip(episode_relevances.iter_mut()) {
            if let Some(ep_day) = parse_iso_date_day(&ep.timestamp) {
                if temporal_ranges.iter().any(|r| r.contains_day(ep_day)) {
                    *rel = (*rel + 0.55).clamp(0.0, 1.0);
                }
            }
        }
    }

    // --- #1, #4, #10 Score & partition User Facts (Sticky + Relevance-Repaired Decay) ---
    let mut scored_facts: Vec<(&UserFact, f64, f64)> = all_facts
        .iter()
        .copied()
        .enumerate()
        .map(|(i, fact)| {
            let age_days = parse_iso_date_day(&fact.last_updated)
                .map_or(0.0, |d| (today_day - d).max(0) as f64);
            let base_imp = (0.35 + 0.15 * (fact.strength.clamp(1, 6) as f64)).clamp(0.35, 1.25);
            let recall_cnt = u32::try_from(fact.strength.saturating_sub(1).max(0)).unwrap_or(0);
            let intrinsic = intrinsic_score(base_imp, None);
            let familiar =
                decay_score(base_imp, recall_cnt, age_days, DEFAULT_HALF_LIFE_DAYS, None);
            let rel = fact_relevances.get(i).copied().unwrap_or(0.0);
            let rank_score = relevance_repaired_score(intrinsic, familiar, rel);
            (fact, rank_score, rel)
        })
        .collect();

    // Sort by V1 comparator first for deterministic tie-breaking, then by CharacterMemory score.
    let v1_sorted_facts = stable_sorted(&all_facts, &mut |a, b| compare_facts(a, b, collator));
    let mut selected_facts: Vec<&UserFact> = Vec::with_capacity(PROMPT_FACT_LIMIT);

    // Partition sticky slots (top strength facts) vs relevance-driven slots (`structured.py`).
    for &sticky in v1_sorted_facts.iter().take(STICKY_FACT_SLOTS) {
        selected_facts.push(sticky);
    }
    scored_facts.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| compare_facts(a.0, b.0, collator))
    });
    for (fact, _, _) in &scored_facts {
        if selected_facts.len() >= PROMPT_FACT_LIMIT {
            break;
        }
        if !selected_facts
            .iter()
            .any(|existing| std::ptr::eq(*existing, *fact))
        {
            selected_facts.push(fact);
        }
    }

    // --- #1, #3, #5 Score Episodic Memories (Emotion Impact + Mood Congruence + Decay Repair) ---
    let mut scored_episodes: Vec<(&EpisodicMemory, f64, f64)> = all_episodes
        .iter()
        .copied()
        .enumerate()
        .map(|(i, ep)| {
            let age_days =
                parse_iso_date_day(&ep.timestamp).map_or(0.0, |d| (today_day - d).max(0) as f64);
            let ep_emotion = EmotionVector::infer_from_text_and_tags(&ep.summary, &ep.tags, None);
            let impact = ep_emotion.emotional_impact();
            let mood_sim = current_mood.similarity(ep_emotion);
            let base_imp = 0.60 + 0.15 * mood_sim;
            let intrinsic = intrinsic_score(base_imp, Some(impact));
            let familiar = decay_score(
                base_imp,
                ep.tags.len() as u32,
                age_days,
                DEFAULT_HALF_LIFE_DAYS,
                Some(impact),
            );
            let rel = episode_relevances.get(i).copied().unwrap_or(0.0);
            let rank_score = relevance_repaired_score(intrinsic, familiar, rel);
            (ep, rank_score, rel)
        })
        .collect();

    scored_episodes.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| compare_episodes(a.0, b.0, collator))
    });
    let selected_episodes: Vec<&EpisodicMemory> = scored_episodes
        .iter()
        .take(PROMPT_EPISODE_LIMIT)
        .map(|(ep, _, _)| *ep)
        .collect();

    // --- Assemble base sections with dynamic recall ---
    let mut lines: Vec<String> = Vec::new();
    let raw_name = js_trim(memory.user_name.as_deref().unwrap_or(""));
    let prompt_name = if raw_name.is_empty() || raw_name.to_lowercase() == "salieri" {
        super::DEFAULT_PROMPT_NAME
    } else {
        raw_name
    };
    lines.push(format!(
        "【ユーザー情報】ユーザーのことは「{prompt_name}」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。"
    ));
    lines.push(format!(
        "【紅莉栖の人物像・行動指針（Core Profile）】\n{KURISU_CORE_PERSONA_ANCHOR}"
    ));

    // #12 User Summary (synthesized rolling user profile when 3+ facts exist)
    if v1_sorted_facts.len() >= 3 {
        let profile_items: Vec<&str> = v1_sorted_facts
            .iter()
            .take(4)
            .map(|f| f.content.as_str())
            .collect();
        lines.push(format!(
            "【ユーザー人物像サマリー】{prompt_name}の特徴: {}",
            profile_items.join(" / ")
        ));
    }

    // #6 User Directives (standing rules / promises)
    let directives: Vec<&UserFact> = v1_sorted_facts
        .iter()
        .copied()
        .filter(|f| is_user_directive(&f.content))
        .take(PROMPT_DIRECTIVE_LIMIT)
        .collect();
    if !directives.is_empty() {
        lines.push("【ユーザーからの継続指示・約束（最優先で遵守）】".to_owned());
        for dir in directives {
            lines.push(format!("- {}", dir.content));
        }
    }

    // #4 User Facts (Sticky + Contextual BM25/Decay selected)
    if !selected_facts.is_empty() {
        lines.push("【ユーザーについて知っていること（重要度順）】".to_owned());
        for fact in &selected_facts {
            lines.push(format!(
                "- {}（強度:{} / 信頼度:{}）",
                fact.content,
                fact.strength,
                certainty_label(fact.strength)
            ));
        }
    }

    // #5 Episodic Memory (Emotion-weighted + Contextual BM25/KG selected)
    let summaries = memory.conversation_summaries.as_deref().unwrap_or(&[]);
    if !selected_episodes.is_empty() {
        lines.push("【最近のエピソード記憶】".to_owned());
        for episode in &selected_episodes {
            let suffix = if episode.tags.is_empty() {
                String::new()
            } else {
                format!("（タグ: {}）", episode.tags.join(", "))
            };
            lines.push(format!(
                "- [{}] {}{}",
                episode.timestamp, episode.summary, suffix
            ));
        }
    } else if !summaries.is_empty() {
        lines.push("【過去の会話の記憶】".to_owned());
        let start = summaries.len().saturating_sub(super::PROMPT_SUMMARY_LIMIT);
        for summary in &summaries[start..] {
            lines.push(format!("- {summary}"));
        }
    }

    // #13 Calendar / Prospective Events
    let mut calendar_items: Vec<String> = Vec::new();
    for fact in &v1_sorted_facts {
        if is_calendar_entry(&fact.content, &[]) && calendar_items.len() < PROMPT_CALENDAR_LIMIT {
            calendar_items.push(format!("- [記録日:{}] {}", fact.last_updated, fact.content));
        }
    }
    for ep in &all_episodes {
        if is_calendar_entry(&ep.summary, &ep.tags) && calendar_items.len() < PROMPT_CALENDAR_LIMIT
        {
            calendar_items.push(format!("- [{}] {}", ep.timestamp, ep.summary));
        }
    }
    if !calendar_items.is_empty() {
        lines.push("【カレンダー・予定・記念日の記憶】".to_owned());
        lines.extend(calendar_items);
    }

    // #8 Relationship, Character Info & Dialogue Style RAG (retrieved dynamically per query)
    let relationship_chunks =
        recall_character_chunks(KURISU_RELATIONSHIPS, &queries, PROMPT_RELATIONSHIP_LIMIT);
    if !relationship_chunks.is_empty() {
        lines.push("【関連する人間関係（Relationships）】".to_owned());
        for chunk in relationship_chunks {
            lines.push(format!("- {chunk}"));
        }
    }
    let lore_chunks = recall_character_chunks(KURISU_CHARACTER_INFO, &queries, PROMPT_LORE_LIMIT);
    if !lore_chunks.is_empty() {
        lines.push("【関連する知識・記憶データ（Character Info）】".to_owned());
        for chunk in lore_chunks {
            lines.push(format!("- {chunk}"));
        }
    }
    let style_chunks = recall_character_chunks(KURISU_DIALOGUE_STYLE, &queries, PROMPT_STYLE_LIMIT);
    if !style_chunks.is_empty() {
        lines.push("【関連する口調・反応ガイド（Dialogue Style）】".to_owned());
        for chunk in style_chunks {
            lines.push(format!("- {chunk}"));
        }
    }

    // #11 Emotion & Relationship Affinity tracking
    let total = memory.total_interactions.unwrap_or(0);
    if total > 0 || !all_facts.is_empty() || !all_episodes.is_empty() {
        let (rel_label, rel_score) =
            relationship_affinity_label(total, all_facts.len(), all_episodes.len(), current_mood);
        lines.push(format!(
            "【関係性・親密度】{rel_label}（親密度スコア: {rel_score}/100）"
        ));
    }

    if let Some(date) = memory
        .last_session_date
        .as_deref()
        .filter(|d| !d.is_empty())
    {
        lines.push(format!("【前回のセッション】{date}"));
    }
    if total > 0 {
        lines.push(format!("【累計やりとり回数】{total}回"));
    }
    if recent_emotions.len() >= super::EMOTION_TREND_MIN {
        lines.push(format!(
            "【最近の感情傾向】{}（同じ感情が続きすぎないように意識して）",
            recent_emotions.join("→")
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{LocalDateTime, MemoryManager};
    use crate::protocol::ChatMessage;
    use crate::random_events::SequenceRng;
    use crate::storage::MemoryStore;

    const NOW: LocalDateTime = LocalDateTime::new(2026, 11, 22, 20, 0, 0);

    #[test]
    fn decay_and_relevance_repair_follow_character_memory_formula() {
        // Feature #1: decay.py formula
        let base = 1.0;
        let impact = Some(0.5);
        let intrinsic = intrinsic_score(base, impact);
        assert!((intrinsic - 1.5).abs() < 1e-9);

        // Fresh memory at age 0 equals intrinsic
        let fresh = decay_score(base, 0, 0.0, 30.0, impact);
        assert!((fresh - intrinsic).abs() < 1e-9);

        // Decayed memory after 45 days (1 half-life since 30 * (1 + 0.5) = 45)
        let aged = decay_score(base, 0, 45.0, 30.0, impact);
        assert!(aged < intrinsic && aged > 0.5);

        // Exposure familiarity boost (bounded at 10% of intrinsic)
        let familiar = decay_score(base, 10, 45.0, 30.0, impact);
        assert!((familiar - (aged + 1.5 * 0.10 * 0.5)).abs() < 1e-9);

        // 110% relevance repair restores lost salience when relevance = 1.0
        let repaired = relevance_repaired_score(intrinsic, familiar, 1.0);
        let expected = familiar + 1.10 * (intrinsic - familiar);
        assert!((repaired - expected).abs() < 1e-9);
        assert!(repaired > intrinsic);
    }

    #[test]
    fn emotion_vectors_impact_and_mood_congruence() {
        // Feature #3 & #11: emotion_vectors.py & emotion.py
        let blush = EmotionVector::from_tag("BLUSH");
        let angry = EmotionVector::from_tag("ANGRY");
        let smile = EmotionVector::from_tag("SMILE");

        assert!(blush.emotional_impact() > 0.8);
        assert!(blush.similarity(smile) > blush.similarity(angry));

        let mood = current_mood_vector(&["SMILE".into(), "BLUSH".into(), "WINK".into()]);
        assert!(mood.affection > 0.6);
        let (label, score) = relationship_affinity_label(35, 6, 5, mood);
        assert!(score >= 80, "score={score}");
        assert!(label.contains("深い信頼関係"));
    }

    #[test]
    fn bm25_rrf_kg_temporal_directives_lore_and_calendar_work_end_to_end() {
        // Features #2, #4, #5, #6, #7, #8, #9, #10, #12, #13
        let store = MemoryStore::new();
        let m = MemoryManager::new(&store, &NOW);

        // Add 8 facts (only 6 fit in PROMPT_FACT_LIMIT)
        for i in 1..=6 {
            m.add_user_fact(
                &format!("一般的な事実{i}"),
                &LocalDateTime::new(2026, 11, 20, 12, 0, 0),
            );
        }
        // An older fact about quantum physics (would be dropped by V1's top-6 cutoff!)
        m.add_user_fact(
            "量子力学とブラックホール蒸発の研究をしている",
            &LocalDateTime::new(2026, 10, 1, 12, 0, 0),
        );
        // Feature #6: A standing directive
        m.add_user_fact("敬語は使わないでタメ口で話してという約束", &NOW);
        // Feature #13: A calendar fact
        m.add_user_fact("来週の金曜日に学会発表の予定がある", &NOW);

        // Add 6 episodes (only 4 fit in PROMPT_EPISODE_LIMIT)
        for i in 1..=5 {
            m.add_episodic_memory(
                &format!("普通の雑談をした{i}"),
                &["雑談"],
                &LocalDateTime::new(2026, 11, 22, 10, 0, 0),
            );
        }
        // An episode from yesterday (2026-11-21) with tag "量子" (linked via KG to the old fact)
        m.add_episodic_memory(
            "深夜までタイムマシンと量子テレポーテーションの議論で盛り上がった",
            &["量子", "研究"],
            &LocalDateTime::new(2026, 11, 21, 23, 0, 0),
        );

        // User asks about yesterday's quantum time machine talk + nullpo
        let history = vec![
            ChatMessage::system("BASE"),
            ChatMessage::user("昨日話した量子とタイムマシンの件、覚えてる？あと、ぬるぽ"),
        ];
        let mut rng = SequenceRng::new([0.0]);
        let updated = m.update_system_prompt(&history, "BASE", 1.0, &NOW, &mut rng);
        let sys = &updated[0].content;

        // #4 & #1/#2/#7: The old quantum fact was recalled into top-6 via BM25 + Relevance Repair!
        assert!(
            sys.contains("量子力学とブラックホール蒸発の研究をしている"),
            "old relevant fact recalled: {sys}"
        );
        // #5 & #9: Yesterday's episode was recalled into top-4 via BM25 + Temporal ("昨日") + KG!
        assert!(
            sys.contains("深夜までタイムマシンと量子テレポーテーションの議論で盛り上がった"),
            "yesterday's episode recalled: {sys}"
        );
        // #6: User directive section is rendered
        assert!(
            sys.contains("【ユーザーからの継続指示・約束（最優先で遵守）】"),
            "directives section present: {sys}"
        );
        // #8: Character info (Time Machine / @channel) and Dialogue Style (nullpo -> Gah!) recalled
        assert!(
            sys.contains("【関連する知識・記憶データ（Character Info）】"),
            "character info section present: {sys}"
        );
        assert!(
            sys.contains("【関連する口調・反応ガイド（Dialogue Style）】")
                && sys.contains("ガッ！"),
            "dialogue style section present: {sys}"
        );
        // #11: Relationship affinity section
        assert!(
            sys.contains("【関係性・親密度】"),
            "affinity section present: {sys}"
        );
        // #12: User summary section
        assert!(
            sys.contains("【ユーザー人物像サマリー】"),
            "user summary section present: {sys}"
        );
        // #13: Calendar section
        assert!(
            sys.contains("【カレンダー・予定・記念日の記憶】")
                && sys.contains("来週の金曜日に学会発表の予定がある"),
            "calendar section present: {sys}"
        );
    }

    #[test]
    fn kurisu_profile_and_relationships_recall_cover_sg_and_sg0() {
        let store = MemoryStore::new();
        let m = MemoryManager::new(&store, &NOW);

        // 0-turn memory_context remains V1 bit-identical (no Core Profile injected)
        assert!(!m
            .memory_context()
            .contains("【紅莉栖の人物像・行動指針（Core Profile）】"));

        // Relationship recall checks across SG & SG0 cast
        let cases = [
            ("真帆先輩とモーツァルトについて話して", "比屋定真帆"),
            ("レスキネン教授について教えて", "アレクシス・レスキネン"),
            ("レイエス教授との件はどうだった？", "ジュディ・レイエス"),
            ("パパや中鉢の論文とマイフォークの話", "牧瀬章一"),
            ("ママとのジェットコースターの思い出", "スリーパーホールド"),
            ("まゆりはどんな子？", "椎名まゆり"),
            ("橋田って本当にHENTAIだよね", "橋田至"),
            ("阿万音鈴羽とジョン・タイターについて", "阿万音鈴羽"),
            ("漆原るかさんってどんな人？", "漆原るか"),
            ("フェイリスさんはメイド喫茶のオーナー？", "フェイリス"),
            ("桐生萌郁さんが負傷した時どうした？", "桐生萌郁"),
            ("店長のブラウン管と綯ちゃんのこと", "天王寺裕吾"),
            ("椎名かがりの行方を探してる", "椎名かがり"),
            ("由季さんやカエデ、フブキについて", "カエデ"),
            ("岡部倫太郎（オカリン）のことどう思う？", "岡部倫太郎"),
        ];
        for (prompt, expected_substring) in cases {
            let history = vec![ChatMessage::system("BASE"), ChatMessage::user(prompt)];
            let mut rng = SequenceRng::new([0.0]);
            let updated = m.update_system_prompt(&history, "BASE", 0.0, &NOW, &mut rng);
            let sys = &updated[0].content;
            assert!(
                sys.contains("【紅莉栖の人物像・行動指針（Core Profile）】"),
                "core profile missing for `{prompt}`"
            );
            assert!(
                sys.contains("【関連する人間関係（Relationships）】")
                    && sys.contains(expected_substring),
                "expected `{expected_substring}` for `{prompt}` in:\n{sys}"
            );
        }

        // Natural non-exact-keyword phrasing via BM25 + neutral greeting noise guard
        let cold_coffee = recall_character_chunks(
            KURISU_CHARACTER_INFO,
            &[("寒い日に飲みたいものってある？".into(), 1.0)],
            2,
        );
        assert!(
            cold_coffee.iter().any(|t| t.contains("食の嗜好")),
            "BM25 should recall food/coffee chunk on natural wording: {cold_coffee:?}"
        );
        let neutral =
            recall_character_chunks(KURISU_RELATIONSHIPS, &[("こんにちは".into(), 1.0)], 3);
        assert!(
            neutral.is_empty(),
            "neutral greeting must not false-trigger relationships: {neutral:?}"
        );
    }
}
