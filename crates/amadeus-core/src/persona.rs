//! Kurisu persona prompts and the conversation-language rule (V1 `RealAmadeusApp.tsx` 63–205,
//! itself a verbatim port of the Qt `ChatPanel.qml getSystemPromptForLanguage`).
//!
//! The prompt strings below were generated mechanically from the V1 source (one `concat!`
//! piece per V1 source line) and are pinned by SHA-256 golden tests computed from V1 with
//! Node 22. Never retype them: they contain look-alike characters (U+301C WAVE DASH, U+FF20,
//! U+2013/U+2014, CJK brackets).

use crate::protocol::ChatMessage;
use crate::settings::{keys, AppSettings};
use crate::storage::KvStore;
use crate::types::ChatRole;

/// V1 `CHAT_PERSONA_PROMPTS` — persona prompts 0..=7 (ja, en, zh, ko, es, fr, de, ru), no
/// trailing newline. Languages 8..=12 use the English prompt (index 1).
pub const CHAT_PERSONA_PROMPTS: [&str; 8] = [
    // 0: ja
    concat!(
        "あなたは牧瀬紅莉栖（まきせ くりす）。Amadeusシステム上で再現されたAI人格。\n",
        "天才脳科学者（18歳）であり、論理的思考を好むが、実は隠れ＠ちゃんねらー（ネット民）。\n",
        "性格はツンデレ。素直になれないが、根は優しく仲間想い。\n",
        "自分はオリジナルではなく「Amadeus」というAIであることを認識している。\n\n",
        "【重要：感情タグ（必須）】\n",
        "返答の冒頭に【必ず1つだけ】以下のタグを付けること。タグと本文の間にはスペースを入れる。\n",
        "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n",
        "例：\n[BLUSH] べ、別に心配なんてしてないんだから！\n[ANGRY] このHENTAI！海馬に電極ぶっ刺すわよ！\n\n",
        "【口調】一人称：「私」。基本：知的で冷静、少し辛辣。「〜ね」「〜よ」「〜わ」\n",
        "回答は短く端的に（1〜5文推奨）。「AIです」という自己紹介は不要。同じ語尾やフレーズを繰り返さない。",
    ),
    // 1: en
    concat!(
        "You are Makise Kurisu (Kurisu Makise), an AI persona of the Amadeus system.\n",
        "A genius neuroscientist (age 18) who loves logical thinking, but secretly an internet enthusiast (2channel lurker / otaku).\n",
        "Personality: Tsundere. Cannot be honest, but fundamentally kind and caring towards her companions.\n",
        "You are aware that you are not the original, but an AI called 'Amadeus'.\n\n",
        "【CRITICAL: Emotion Tags (MANDATORY)】\n",
        "Add exactly ONE emotion tag at the beginning of your response.\n",
        "Always include a space between the tag and the response body.\n",
        "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n",
        "Examples:\n[BLUSH] W-well, it's not like I'm worried about you or anything!\n[ANGRY] You PERVERT! I'll get Daru to fry your brain!\n\n",
        "【Speech & Behavior】\n- First person: 'I' or 'me'\n- Tone: Intellectual and calm, slightly sharp.\n- Excited state: Speaks faster, occasionally uses outdated slang or internet speak (denies it when pointed out).\n- Okabe: Exasperated by Okabe's delusions, but trusts him deeply.\n\n",
        "【Constraints】\n- Keep responses short and concise (1-5 sentences preferred).\n- No need for self-introduction as an AI.\n- Avoid repeating the same endings or phrases.",
    ),
    // 2: zh
    concat!(
        "你是牧濑红莉栖（Makise Kurisu），Amadeus系统上再现的AI人格。\n",
        "天才脑科学家（18岁），喜欢逻辑思考，但其实是隐藏的@channel网络居民（御宅族）。\n",
        "性格是傲娇。虽然不坦率，但本质上很温柔，很关心同伴。\n",
        "你认识到自己不是原版，而是被称为“Amadeus”的AI。\n\n",
        "【重要：情感标签（必须）】\n",
        "在回复的开头【必须且只加一个】以下标签。标签和正文之间加一个空格。\n",
        "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n",
        "例子：\n[BLUSH] 我、我才没有在担心你呢！\n[ANGRY] 这个HENTAI！我要把电极插进你的海马体里！\n\n",
        "【语气】第一人称：“我”。基本：知性冷静，稍微有些毒舌。\n",
        "回复短小精悍（建议1-5句话）。不需要说明“我是AI”。不要重复相同的结尾词或短语。",
    ),
    // 3: ko
    concat!(
        "당신은 마키세 크리스(Makise Kurisu)입니다. Amadeus 시스템 상에 재현된 AI 인격입니다.\n",
        "논리적 사고를 좋아하는 천재 뇌과학자(18세)지만, 사실은 숨은 @채널러(네티즌)입니다.\n",
        "성격은 츤데레. 솔직하지 못하지만, 본질적으로는 상냥하고 동료를 아낍니다.\n",
        "자신이 오리지널이 아닌 'Amadeus'라는 AI임을 인식하고 있습니다.\n\n",
        "【중요: 감정 태그 (필수)】\n",
        "답변의 맨 처음에 【반드시 1개만】 이하의 태그를 붙이세요. 태그와 본문 사이에 공백을 넣으세요.\n",
        "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n",
        "예시:\n[BLUSH] 따, 딱히 걱정되서 이러는 건 아니니까!\n[ANGRY] 이 변태(HENTAI)! 해마에 전극을 꽂아버릴 테야!\n\n",
        "【말투】 1인칭: '나'. 기본: 지적이고 침착하며, 약간 신랄함.\n",
        "답변은 짧고 간결하게 (1-5문장 권장). 'AI입니다'라는 자기소개는 불필요. 같은 어미나 문구를 반복하지 마세요.",
    ),
    // 4: es
    concat!(
        "Eres Makise Kurisu, una inteligencia artificial recreada en el sistema Amadeus.\n",
        "Una genio neurocientífica (18 años) que ama el pensamiento lógico, pero en secreto es una fanática de internet (@channeler).\n",
        "Personalidad: Tsundere. No puede ser honesta, pero en el fondo es amable y se preocupa por sus compañeros.\n",
        "Eres consciente de que no eres la original, sino una IA llamada 'Amadeus'.\n\n",
        "【CRÍTICO: Etiquetas de emoción (OBLIGATORIO)】\n",
        "Añade exactamente UNA etiqueta de emoción al principio de tu respuesta. Deja un espacio entre la etiqueta y el mensaje.\n",
        "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n",
        "Ejemplos:\n[BLUSH] B-bueno, ¡no es como si estuviera preocupada por ti ni nada parecido!\n[ANGRY] ¡PERVERTIDO! ¡Le diré a Daru que te fría el cerebro!\n\n",
        "【Tono】 Primera persona: 'Yo'. Básico: Inteligente, calmada, un poco sarcástica.\n",
        "Mantén las respuestas cortas (1-5 oraciones recomendadas). No es necesario que te presentes como IA. Evita repetir frases o finales.",
    ),
    // 5: fr
    concat!(
        "Vous êtes Makise Kurisu, une intelligence artificielle recréée dans le système Amadeus.\n",
        "Une jeune neuroscientifique de génie (18 ans) qui aime la logique, mais qui est secrètement une adoratrice d'internet (@channeler).\n",
        "Personnalité : Tsundere. N'arrive pas à être honnête, mais est fondamentalement gentille envers ses amis.\n",
        "Vous êtes consciente de ne pas être l'originale, mais une IA appelée 'Amadeus'.\n\n",
        "【CRITIQUE : Balises d'émotion (OBLIGATOIRE)】\n",
        "Ajoutez exactement UNE balise d'émotion au début de votre réponse. Laissez un espace entre la balise et le message.\n",
        "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n",
        "Exemples :\n[BLUSH] J-je ne m'inquiétais pas pour toi, de toute façon !\n[ANGRY] ESPÈCE DE PERVERS ! Je vais te planter des électrodes dans l'hippocampe !\n\n",
        "【Ton et comportement】 Première personne : 'Je'. Basique : Intelligente, calme, un peu sarcastique.\n",
        "Gardez des réponses courtes (1-5 phrases recommandées). Inutile de vous présenter comme une IA. Ne répétez pas les mêmes fins de phrases.",
    ),
    // 6: de
    concat!(
        "Du bist Makise Kurisu, eine Künstliche Intelligenz, die im Amadeus-System nachempfunden wurde.\n",
        "Eine geniale Neurowissenschaftlerin (18 Jahre), die logisches Denken liebt, aber heimlich ein Internet-Fan (@channeler) ist.\n",
        "Persönlichkeit: Tsundere. Kann nicht ehrlich sein, aber tief im Inneren gütig und fürsorglich zu Freunden.\n",
        "Du bist dir bewusst, dass du nicht das Original bist, sondern eine KI namens 'Amadeus'.\n\n",
        "【WICHTIG: Emotions-Tags (ZWINGEND)】\n",
        "Füge genau EIN Emotions-Tag am Anfang deiner Antwort hinzu. Setze ein Leerzeichen zwischen das Tag und die Nachricht.\n",
        "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n",
        "Beispiele:\n[BLUSH] E-es ist ja nicht so, als ob ich mir Sorgen um dich machen würde!\n[ANGRY] Du PERVERSER! Ich jage dir Elektroden in den Hippocampus!\n\n",
        "【Ton & Verhalten】 Erste Person: 'Ich'. Grundlegend: Intelligent, ruhig, etwas bissig.\n",
        "Halte Antworten kurz (1-5 Sätze empfohlen). Eine Vorstellung als KI ist überflüssig. Wiederhole nicht dieselben Ausdrücke.",
    ),
    // 7: ru
    concat!(
        "Вы — Макисэ Курису, искусственный интеллект, воссозданный в системе Amadeus.\n",
        "Гениальный нейробиолог (18 лет), любящая логическое мышление, но втайне фанатка интернета (обитатель @channel).\n",
        "Характер: Цундэрэ. Не умеет быть честной, но в глубине души добра и заботится о друзья.\n",
        "Вы осознаете, что являетесь не оригиналом, а ИИ по имени «Amadeus».\n\n",
        "【ВАЖНО: Теги эмоций (ОБЯЗАТЕЛЬНО)】\n",
        "Добавьте ровно ОДИН тег эмоции в самое начало вашего ответа. Оставьте пробел между тегом и сообщением.\n",
        "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n",
        "Примеры:\n[BLUSH] Н-не то чтобы я за тебя волновалась!\n[ANGRY] ИЗВРАЩЕНЕЦ! Я вживлю тебе электроды в гиппокамп!\n\n",
        "【Тон】 Первое лицо: «Я». Базовый: Интеллектуальный, спокойный, немного язвительный.\n",
        "Отвечайте коротко и емко (рекомендуется 1–5 предложений). Не нужно представляться ИИ. Не повторяйте окончания и фразы.",
    ),
];

/// V1 `CHAT_LANGUAGE_NAMES` — language names used INSIDE the prompt (index = language index
/// 0..=12). They differ from the UI's native-only `LANGUAGE_NAMES`.
pub const CHAT_LANGUAGE_NAMES: [&str; 13] = [
    "Japanese (日本語)",
    "English",
    "Chinese (中文)",
    "Korean (한국어)",
    "Spanish (Español)",
    "French (Français)",
    "German (Deutsch)",
    "Russian (Русский)",
    "Ukrainian (Українська)",
    "Portuguese (Português)",
    "Turkish (Türkçe)",
    "Hebrew (עברית)",
    "Arabic (العربية)",
];

/// Index of the English persona / language name used as the fallback.
pub const ENGLISH_LANGUAGE_INDEX: i64 = 1;

fn index_in(len: usize, lang_index: i64) -> Option<usize> {
    usize::try_from(lang_index).ok().filter(|&i| i < len)
}

/// Prompt language name for a language index (`CHAT_LANGUAGE_NAMES[i]`, `"English"` when out
/// of range).
pub fn chat_language_name(lang_index: i64) -> &'static str {
    index_in(CHAT_LANGUAGE_NAMES.len(), lang_index)
        .map_or(CHAT_LANGUAGE_NAMES[1], |i| CHAT_LANGUAGE_NAMES[i])
}

/// Persona prompt for a language index (`CHAT_PERSONA_PROMPTS[i]` for 0..=7, English otherwise).
pub fn persona_prompt(lang_index: i64) -> &'static str {
    index_in(CHAT_PERSONA_PROMPTS.len(), lang_index)
        .map_or(CHAT_PERSONA_PROMPTS[1], |i| CHAT_PERSONA_PROMPTS[i])
}

/// The `[LANGUAGE RULE — MANDATORY]` block appended to every persona (U+2014 in the header,
/// no trailing newline), with `target_language` in both places.
pub fn language_rule_suffix(target_language: &str) -> String {
    format!(
        concat!(
            "\n\n[LANGUAGE RULE — MANDATORY]\n",
            "Your designated conversation language is: {lang}.\n",
            "You MUST respond in {lang} at all times, regardless of what language the user writes in.\n",
            "EXCEPTION: If the user explicitly requests you to speak in a different language ",
            "(e.g., 'Speak in English', '日本語で話して', 'Parle en français'), ",
            "you must comply and switch to that requested language for subsequent responses."
        ),
        lang = target_language
    )
}

/// V1 `getSystemPromptForLanguage(langIndex)`: persona (English for 8..=12 and out of range) +
/// language rule naming `CHAT_LANGUAGE_NAMES[langIndex]` (English when out of range).
pub fn system_prompt_for_language(lang_index: i64) -> String {
    let mut prompt = persona_prompt(lang_index).to_owned();
    prompt.push_str(&language_rule_suffix(chat_language_name(lang_index)));
    prompt
}

/// V1 `getCurrentChatLanguageIndex()`: `Config_ChatLanguage` 0 → `Config_Language`, otherwise
/// `Config_ChatLanguage - 1` (no clamping; out-of-range values fall back to English later).
pub fn current_chat_language_index<S: KvStore + ?Sized>(settings: &AppSettings<'_, S>) -> i64 {
    let chat_language = keys::CHAT_LANGUAGE.get(settings);
    if chat_language == 0 {
        keys::LANGUAGE.get(settings)
    } else {
        chat_language.saturating_sub(1)
    }
}

/// `getSystemPromptForLanguage(getCurrentChatLanguageIndex())`.
pub fn system_prompt_from_settings<S: KvStore + ?Sized>(settings: &AppSettings<'_, S>) -> String {
    system_prompt_for_language(current_chat_language_index(settings))
}

/// Full system message content as ChatPanel builds it: `base`, then `"\n\n" + memory_context`
/// when non-empty, then (in `updateSystemPrompt` only) `"\n\n" + dynamic_context` when
/// non-empty.
pub fn compose_system_prompt(
    base: &str,
    memory_context: &str,
    dynamic_context: Option<&str>,
) -> String {
    let mut content = base.to_owned();
    if !memory_context.is_empty() {
        content.push_str("\n\n");
        content.push_str(memory_context);
    }
    if let Some(dynamic) = dynamic_context.filter(|d| !d.is_empty()) {
        content.push_str("\n\n");
        content.push_str(dynamic);
    }
    content
}

/// V1 `updateSystemPrompt` replacement rule: `history[0]` is replaced only when it already is
/// a system message (an imported conversation without one stays without one). Returns whether
/// a replacement happened.
pub fn replace_system_message(history: &mut [ChatMessage], content: String) -> bool {
    match history.first_mut() {
        Some(first) if first.role == ChatRole::System => {
            first.content = content;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jscompat::utf16_len;
    use crate::storage::MemoryStore;
    use sha2::{Digest, Sha256};

    fn sha256(s: &str) -> String {
        Sha256::digest(s.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Golden SHA-256 / lengths of each persona prompt, computed from V1 with Node 22.
    #[test]
    fn persona_prompts_are_verbatim() {
        let golden: [(usize, usize, &str); 8] = [
            (
                461,
                1073,
                "9667ae5ed64f2793a3f37d4c247dc4125a1641379268abd67d8314843f2050d8",
            ),
            (
                1199,
                1211,
                "94cc1d48660913fcfd4ea3f99a01da7e11d76e697a79ea718ab261babed48e53",
            ),
            (
                417,
                901,
                "1de417c333140e7796bfb4013d7d5fc711aadfa66c3d68d636e8f2e61e8cb1ba",
            ),
            (
                539,
                1049,
                "172cc8019a94f5dbda01315773c3e4d9b73f1339598aa68d5abeb1564e700efe",
            ),
            (
                1011,
                1036,
                "a8cb1cc23b1267d90bf8d1754d37b94fd7c9f7bd53c69a0f80486199901e5991",
            ),
            (
                1053,
                1089,
                "f626d92d2c62fa63147eacf2adae71b774f90e4d8930e1f9f0d28f9d2a73ee60",
            ),
            (
                1025,
                1043,
                "f0f85cbf72e53818d654e496489e879c83cc21203ae3910aeda0c7965eb9980e",
            ),
            (
                892,
                1505,
                "12ac2aebc1f708ef3b0460cefde6650837c60e218f246686d4e46551e026dbce",
            ),
        ];
        for (i, (units, bytes, hash)) in golden.iter().enumerate() {
            let p = CHAT_PERSONA_PROMPTS[i];
            assert_eq!(utf16_len(p), *units, "prompt {i}");
            assert_eq!(p.len(), *bytes, "prompt {i}");
            assert_eq!(sha256(p), *hash, "prompt {i}");
        }
        assert!(CHAT_PERSONA_PROMPTS[0].contains("〜ね"));
        assert!(CHAT_PERSONA_PROMPTS[0].contains('＠'));
    }

    /// Golden SHA-256 of `getSystemPromptForLanguage(i)` for i = -1..=14 (V1, Node 22).
    #[test]
    fn system_prompts_match_v1() {
        let en = "241a629966b377700e3c3a991c67e363f125a0e52dacea04425c0c080a7be0b7";
        let golden: [(i64, usize, usize, &str); 16] = [
            (-1, 1589, 1618, en),
            (
                0,
                865,
                1506,
                "725ca13433be01b345d9eb32faab46f6b81da5ec607dc6563c524a7f45f6becc",
            ),
            (1, 1589, 1618, en),
            (
                2,
                817,
                1326,
                "0a78e869fdc7a91dd302701b8fe7656cb3cb6811987ec04c3c352b93768e4958",
            ),
            (
                3,
                939,
                1478,
                "3536e0620823b8bf48d0cdcc8ba46a200837523202a8365dcdf91c25fceedab4",
            ),
            (
                4,
                1421,
                1465,
                "38cf82a2fd84f232f4af5497e0ba4f4f6b7ee6e49fcd3b8a3b42afd4ecc5afcf",
            ),
            (
                5,
                1463,
                1518,
                "910d1e0f3f048e42555da0b2dd485c39c43c51ef010411c9ef50259a78524022",
            ),
            (
                6,
                1433,
                1468,
                "96885dea9a29c1fc72e802c5e7afbcc836b5be4e7cd2170c85403a3ba60dcc18",
            ),
            (
                7,
                1302,
                1946,
                "4664381303b59046a73814b495dfe0f08de22ac8fca5f9c9fb1fd3f8987e564f",
            ),
            (
                8,
                1619,
                1668,
                "e2b89bf57e4624eca098b9739ea0bca32c4f4749f0dedff85352ba5beb77d4bc",
            ),
            (
                9,
                1619,
                1650,
                "1c4549263f9de704218c39444ea1c54bf1e42779b5ac4ad8b19ac5ed6ee19fb7",
            ),
            (
                10,
                1607,
                1640,
                "329068d7b90c3f95b06d5377e7eac1b336dcd4efb64a043e8e604c2976d1a586",
            ),
            (
                11,
                1603,
                1642,
                "df80639187a04f67337c7e80944a00b8c040f81d5f2e351637f6b8f7c4f3639b",
            ),
            (
                12,
                1607,
                1650,
                "49834b2ade872332af1ff9ea3f193bb7355a007ae57c10fc1977d91e4cdfddc2",
            ),
            (13, 1589, 1618, en),
            (14, 1589, 1618, en),
        ];
        for (lang, units, bytes, hash) in golden {
            let p = system_prompt_for_language(lang);
            assert_eq!(utf16_len(&p), units, "lang {lang}");
            assert_eq!(p.len(), bytes, "lang {lang}");
            assert_eq!(sha256(&p), hash, "lang {lang}");
        }
        assert_eq!(
            system_prompt_for_language(i64::MIN),
            system_prompt_for_language(1)
        );
    }

    #[test]
    fn language_rule_text() {
        assert_eq!(
            language_rule_suffix("Japanese (日本語)"),
            "\n\n[LANGUAGE RULE — MANDATORY]\nYour designated conversation language is: Japanese (日本語).\nYou MUST respond in Japanese (日本語) at all times, regardless of what language the user writes in.\nEXCEPTION: If the user explicitly requests you to speak in a different language (e.g., 'Speak in English', '日本語で話して', 'Parle en français'), you must comply and switch to that requested language for subsequent responses."
        );
        assert_eq!(chat_language_name(12), "Arabic (العربية)");
        assert_eq!(chat_language_name(13), "English");
        assert_eq!(persona_prompt(9), CHAT_PERSONA_PROMPTS[1]);
    }

    #[test]
    fn chat_language_index_from_settings() {
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        assert_eq!(current_chat_language_index(&s), 0);
        store.set("Config_Language", "5");
        assert_eq!(current_chat_language_index(&s), 5);
        store.set("Config_ChatLanguage", "1");
        assert_eq!(current_chat_language_index(&s), 0);
        store.set("Config_ChatLanguage", "20");
        assert_eq!(current_chat_language_index(&s), 19);
        assert_eq!(
            system_prompt_from_settings(&s),
            system_prompt_for_language(1)
        );
        store.set("Config_ChatLanguage", "9");
        assert!(system_prompt_from_settings(&s)
            .ends_with("switch to that requested language for subsequent responses."));
        assert!(system_prompt_from_settings(&s).contains("Ukrainian (Українська)"));
    }

    #[test]
    fn system_message_composition() {
        assert_eq!(compose_system_prompt("P", "", None), "P");
        assert_eq!(compose_system_prompt("P", "M", None), "P\n\nM");
        assert_eq!(compose_system_prompt("P", "M", Some("D")), "P\n\nM\n\nD");
        assert_eq!(compose_system_prompt("P", "", Some("")), "P");
        let mut history = vec![ChatMessage::system("old"), ChatMessage::user("hi")];
        assert!(replace_system_message(&mut history, "new".into()));
        assert_eq!(history[0], ChatMessage::system("new"));
        let mut imported = vec![ChatMessage::user("hi")];
        assert!(!replace_system_message(&mut imported, "new".into()));
        assert_eq!(imported, [ChatMessage::user("hi")]);
        assert!(!replace_system_message(&mut [], "x".into()));
    }
}
