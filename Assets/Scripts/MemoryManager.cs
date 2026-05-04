using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

/// <summary>
/// Manages short-term and long-term memory for the Amadeus AI system.
/// Short-term: conversation sliding window.
/// Long-term: persistent facts saved as JSON.
/// </summary>
public class MemoryManager : MonoBehaviour
{
    // ─── Configuration ───
    [Header("Memory Settings")]
    public int maxConversationTurns = 30;       // Keep last N user+assistant pairs
    public int summarizeThreshold = 20;         // Summarize when above this many turns
    public int maxLongTermFacts = 50;           // Max stored facts

    // ─── Persistent Data Structure ───
    [System.Serializable]
    public struct FactItem
    {
        public string content;
        public int strength;
        public string lastUpdated;
    }

    [System.Serializable]
    public struct EpisodicItem
    {
        public string timestamp;
        public string summary;
        public List<string> tags;
    }

    [System.Serializable]
    public class KurisuMemory
    {
        public string userName = "";
        public List<FactItem> userFacts = new List<FactItem>();
        public List<string> conversationSummaries = new List<string>();
        public List<EpisodicItem> episodicMemory = new List<EpisodicItem>();
        public string lastSessionDate = "";
        public int totalInteractions = 0;
        public List<string> recentEmotions = new List<string>();
        public List<string> topicsDiscussed = new List<string>();
    }

    private KurisuMemory memory = new KurisuMemory();
    private string savePath;
    private const int maxEpisodicItems = 50;

    // ─── Emotion Tracking ───
    private Queue<string> emotionHistory = new Queue<string>();
    private const int EMOTION_HISTORY_SIZE = 10;

    void Awake()
    {
        savePath = Path.Combine(Application.persistentDataPath, "kurisu_memory.json");
        LoadMemory();
    }

    // ═══════════════════════════════════════════
    //  PUBLIC API
    // ═══════════════════════════════════════════

    /// <summary>
    /// Returns memory context to inject into the system prompt.
    /// </summary>
    public string GetMemoryContext()
    {
        var lines = new List<string>();

        // User identity
        if (!string.IsNullOrEmpty(memory.userName))
            lines.Add($"【ユーザー情報】ユーザーの名前は「{memory.userName}」。");

        // User facts
        if (memory.userFacts.Count > 0)
        {
            lines.Add("【ユーザーについて知っていること（重要度順）】");
            var rankedFacts = new List<FactItem>(memory.userFacts);
            rankedFacts.Sort((a, b) =>
            {
                int strengthCmp = b.strength.CompareTo(a.strength);
                if (strengthCmp != 0) return strengthCmp;
                DateTime ad = ParseIsoDate(a.lastUpdated);
                DateTime bd = ParseIsoDate(b.lastUpdated);
                if (ad != DateTime.MinValue || bd != DateTime.MinValue)
                    return bd.CompareTo(ad);
                return string.Compare(a.content, b.content, StringComparison.Ordinal);
            });

            int factLimit = Mathf.Min(6, rankedFacts.Count);
            for (int i = 0; i < factLimit; i++)
            {
                var fact = rankedFacts[i];
                string certainty = fact.strength >= 4 ? "高" : (fact.strength >= 2 ? "中" : "低");
                lines.Add($"- {fact.content}（強度:{Mathf.Max(1, fact.strength)} / 信頼度:{certainty}）");
            }
        }

        if (memory.episodicMemory.Count > 0)
        {
            lines.Add("【最近のエピソード記憶】");
            var rankedEpisodes = new List<EpisodicItem>(memory.episodicMemory);
            rankedEpisodes.Sort((a, b) =>
            {
                DateTime ad = ParseIsoDate(a.timestamp);
                DateTime bd = ParseIsoDate(b.timestamp);
                if (ad != DateTime.MinValue || bd != DateTime.MinValue)
                    return bd.CompareTo(ad);
                return string.Compare(b.timestamp ?? "", a.timestamp ?? "", StringComparison.Ordinal);
            });

            int episodeLimit = Mathf.Min(4, rankedEpisodes.Count);
            for (int i = 0; i < episodeLimit; i++)
            {
                var episode = rankedEpisodes[i];
                string line = $"- [{episode.timestamp}] {episode.summary}";
                if (episode.tags != null && episode.tags.Count > 0)
                {
                    line += $"（タグ: {string.Join(", ", episode.tags)}）";
                }
                lines.Add(line);
            }
        }

        // Previous conversation summaries
        if (memory.conversationSummaries.Count > 0 && memory.episodicMemory.Count == 0)
        {
            lines.Add("【過去の会話の記憶】");
            // Only include last 3 summaries to save context
            int start = Mathf.Max(0, memory.conversationSummaries.Count - 3);
            for (int i = start; i < memory.conversationSummaries.Count; i++)
                lines.Add($"- {memory.conversationSummaries[i]}");
        }

        // Session info
        if (!string.IsNullOrEmpty(memory.lastSessionDate))
            lines.Add($"【前回のセッション】{memory.lastSessionDate}");

        if (memory.totalInteractions > 0)
            lines.Add($"【累計やりとり回数】{memory.totalInteractions}回");

        // Recent emotion tendency
        if (memory.recentEmotions.Count >= 3)
        {
            lines.Add($"【最近の感情傾向】{string.Join("→", memory.recentEmotions)}（同じ感情が続きすぎないように意識して）");
        }

        return lines.Count > 0 ? string.Join("\n", lines) : "";
    }

    /// <summary>
    /// Records a new emotion and tracks repetition.
    /// Returns true if the same emotion has been used 3+ times in a row.
    /// </summary>
    public bool RecordEmotion(string emotion)
    {
        emotionHistory.Enqueue(emotion);
        if (emotionHistory.Count > EMOTION_HISTORY_SIZE)
            emotionHistory.Dequeue();

        // Track in persistent memory
        memory.recentEmotions.Add(emotion);
        if (memory.recentEmotions.Count > EMOTION_HISTORY_SIZE)
            memory.recentEmotions.RemoveAt(0);

        // Check for 3+ consecutive same emotion
        int consecutive = 0;
        string last = "";
        foreach (var e in emotionHistory)
        {
            if (e == last) consecutive++;
            else { consecutive = 1; last = e; }
        }
        return consecutive >= 3;
    }

    /// <summary>
    /// Increments interaction count and updates session date.
    /// </summary>
    public void RecordInteraction()
    {
        memory.totalInteractions++;
        memory.lastSessionDate = DateTime.Now.ToString("yyyy-MM-dd HH:mm");
        SaveMemory();
    }

    /// <summary>
    /// Stores a fact about the user for long-term memory.
    /// </summary>
    public void AddUserFact(string fact)
    {
        if (string.IsNullOrWhiteSpace(fact)) return;
        string cleaned = fact.Trim();
        string today = DateTime.Now.ToString("yyyy-MM-dd");

        int existingIndex = FindSimilarFactIndex(cleaned);
        if (existingIndex >= 0)
        {
            FactItem existing = memory.userFacts[existingIndex];
            existing.strength = Mathf.Max(1, existing.strength) + 1;
            existing.lastUpdated = today;
            memory.userFacts[existingIndex] = existing;
            SaveMemory();
            return;
        }

        memory.userFacts.Add(new FactItem
        {
            content = cleaned,
            strength = 1,
            lastUpdated = today
        });
        if (memory.userFacts.Count > maxLongTermFacts)
            memory.userFacts.RemoveAt(0);

        SaveMemory();
    }

    /// <summary>
    /// Sets the user's name.
    /// </summary>
    public void SetUserName(string name)
    {
        if (!string.IsNullOrEmpty(name))
        {
            memory.userName = name;
            SaveMemory();
        }
    }

    /// <summary>
    /// Gets the user's stored name (may be empty).
    /// </summary>
    public string GetUserName() => memory.userName;

    /// <summary>
    /// Adds a conversation summary to long-term memory.
    /// </summary>
    public void AddConversationSummary(string summary)
    {
        if (string.IsNullOrEmpty(summary)) return;

        memory.conversationSummaries.Add(summary);
        // Keep at most 10 summaries
        if (memory.conversationSummaries.Count > 10)
            memory.conversationSummaries.RemoveAt(0);

        AddEpisodicMemory(summary, new List<string> { "要約" });
    }

    public void AddEpisodicMemory(string summary, List<string> tags = null)
    {
        if (string.IsNullOrWhiteSpace(summary)) return;

        var item = new EpisodicItem
        {
            timestamp = DateTime.Now.ToString("yyyy-MM-dd"),
            summary = summary.Trim(),
            tags = NormalizeTags(tags)
        };

        memory.episodicMemory.Add(item);
        if (memory.episodicMemory.Count > maxEpisodicItems)
            memory.episodicMemory.RemoveAt(0);

        SaveMemory();
    }

    public int FindSimilarFactIndex(string candidate)
    {
        if (string.IsNullOrWhiteSpace(candidate)) return -1;
        string norm = NormalizeFact(candidate);

        for (int i = 0; i < memory.userFacts.Count; i++)
        {
            var fact = memory.userFacts[i];
            if (NormalizeFact(fact.content) == norm)
                return i;
        }
        return -1;
    }

    /// <summary>
    /// Records a topic that was discussed.
    /// </summary>
    public void AddTopic(string topic)
    {
        if (string.IsNullOrEmpty(topic)) return;
        if (!memory.topicsDiscussed.Contains(topic))
        {
            memory.topicsDiscussed.Add(topic);
            if (memory.topicsDiscussed.Count > 30)
                memory.topicsDiscussed.RemoveAt(0);
        }
    }

    /// <summary>
    /// Manages the conversation history sliding window.
    /// Trims oldest non-system messages when exceeding maxConversationTurns.
    /// Returns a summary of trimmed messages if any were removed.
    /// </summary>
    public string TrimConversationHistory(List<AIService.ChatMessage> history)
    {
        // Count non-system messages
        int nonSystemCount = 0;
        foreach (var msg in history)
        {
            if (msg.role != "system") nonSystemCount++;
        }

        if (nonSystemCount <= maxConversationTurns) return null;

        // Collect messages to summarize (remove oldest pairs)
        int toRemove = nonSystemCount - maxConversationTurns + 4; // Remove extra to avoid frequent trimming
        var removed = new List<string>();
        int removedCount = 0;

        for (int i = history.Count - 1; i >= 0 && removedCount < toRemove; i--)
        {
            // Find non-system messages from the beginning (after system prompt)
            // We iterate from the end to find the start index properly
        }

        // Actually remove from the beginning (after system prompt index 0)
        var toSummarize = new List<string>();
        int idx = 1; // Skip system prompt at index 0
        while (idx < history.Count && removedCount < toRemove)
        {
            toSummarize.Add($"{history[idx].role}: {history[idx].content}");
            history.RemoveAt(idx);
            removedCount++;
        }

        if (toSummarize.Count > 0)
        {
            // Create a simple summary
            string summary = $"[{DateTime.Now:MM/dd HH:mm}の会話] ";
            summary += string.Join(" / ", toSummarize.ConvertAll(s =>
            {
                if (s.Length > 50) return s.Substring(0, 50) + "...";
                return s;
            }));

            if (summary.Length > 300) summary = summary.Substring(0, 300) + "...";

            AddConversationSummary(summary);
            return summary;
        }

        return null;
    }

    /// <summary>
    /// Get the current time-of-day context in Japanese.
    /// </summary>
    public string GetTimeContext()
    {
        int hour = DateTime.Now.Hour;
        if (hour >= 5 && hour < 10) return "朝";
        if (hour >= 10 && hour < 12) return "午前中";
        if (hour >= 12 && hour < 14) return "昼";
        if (hour >= 14 && hour < 17) return "午後";
        if (hour >= 17 && hour < 20) return "夕方";
        if (hour >= 20 && hour < 24) return "夜";
        return "深夜";
    }

    /// <summary>
    /// Generates a dynamic context string to inject into each API call.
    /// </summary>
    public string GetDynamicContext(int turnCount)
    {
        var lines = new List<string>();
        lines.Add($"【現在の状況】時間帯: {GetTimeContext()} / 会話ターン数: {turnCount}");

        // Add variation seed
        int seed = UnityEngine.Random.Range(0, 5);
        string[] moodHints = {
            "（今は少しリラックスしている）",
            "（知的好奇心が高まっている）",
            "（少し眠そう）",
            "（何かを考え込んでいる）",
            "（いつも通りの調子）"
        };
        lines.Add(moodHints[seed]);

        return string.Join("\n", lines);
    }

    /// <summary>
    /// Clears all memory (for debugging/reset).
    /// </summary>
    public void ClearAllMemory()
    {
        memory = new KurisuMemory();
        emotionHistory.Clear();
        SaveMemory();
        Debug.Log("[MemoryManager] All memory cleared.");
    }

    // ═══════════════════════════════════════════
    //  PERSISTENCE
    // ═══════════════════════════════════════════

    private void SaveMemory()
    {
        try
        {
            string json = JsonUtility.ToJson(memory, true);
            File.WriteAllText(savePath, json);
        }
        catch (Exception e)
        {
            Debug.LogWarning($"[MemoryManager] Failed to save memory: {e.Message}");
        }
    }

    private void LoadMemory()
    {
        try
        {
            if (File.Exists(savePath))
            {
                string json = File.ReadAllText(savePath);
                memory = JsonUtility.FromJson<KurisuMemory>(json);
                if (memory == null)
                    memory = new KurisuMemory();
                Debug.Log($"[MemoryManager] Memory loaded. {memory.totalInteractions} total interactions.");

                MigrateLegacyFactsIfNeeded(json);
                NormalizeLoadedData();

                // Restore emotion history
                foreach (var e in memory.recentEmotions)
                    emotionHistory.Enqueue(e);
            }
            else
            {
                Debug.Log("[MemoryManager] No saved memory found. Starting fresh.");
            }
        }
        catch (Exception e)
        {
            Debug.LogWarning($"[MemoryManager] Failed to load memory: {e.Message}");
            memory = new KurisuMemory();
        }
    }

    void OnApplicationQuit()
    {
        SaveMemory();
    }

    private static string NormalizeFact(string fact)
    {
        return string.IsNullOrWhiteSpace(fact) ? string.Empty : fact.Trim().ToLowerInvariant();
    }

    private static DateTime ParseIsoDate(string value)
    {
        if (string.IsNullOrWhiteSpace(value)) return DateTime.MinValue;
        if (DateTime.TryParse(value, out DateTime dt))
            return dt.Date;
        if (value.Length >= 10 && DateTime.TryParse(value.Substring(0, 10), out DateTime dt2))
            return dt2.Date;
        return DateTime.MinValue;
    }

    private static List<string> NormalizeTags(List<string> tags)
    {
        var normalized = new List<string>();
        if (tags == null) return normalized;

        for (int i = 0; i < tags.Count; i++)
        {
            string tag = tags[i];
            if (string.IsNullOrWhiteSpace(tag)) continue;
            string cleaned = tag.Trim();
            bool exists = false;
            for (int j = 0; j < normalized.Count; j++)
            {
                if (string.Equals(normalized[j], cleaned, StringComparison.OrdinalIgnoreCase))
                {
                    exists = true;
                    break;
                }
            }
            if (!exists) normalized.Add(cleaned);
        }
        return normalized;
    }

    [System.Serializable]
    private class LegacyFactsWrapper
    {
        public List<string> userFacts = new List<string>();
    }

    private void MigrateLegacyFactsIfNeeded(string rawJson)
    {
        if (memory.userFacts != null && memory.userFacts.Count > 0)
            return;

        if (string.IsNullOrWhiteSpace(rawJson))
            return;

        try
        {
            LegacyFactsWrapper legacy = JsonUtility.FromJson<LegacyFactsWrapper>(rawJson);
            if (legacy == null || legacy.userFacts == null || legacy.userFacts.Count == 0)
                return;

            string today = DateTime.Now.ToString("yyyy-MM-dd");
            for (int i = 0; i < legacy.userFacts.Count; i++)
            {
                string fact = legacy.userFacts[i];
                if (string.IsNullOrWhiteSpace(fact))
                    continue;
                memory.userFacts.Add(new FactItem
                {
                    content = fact.Trim(),
                    strength = 1,
                    lastUpdated = today
                });
            }
        }
        catch
        {
            // Ignore legacy parse failures and keep current memory state.
        }
    }

    private void NormalizeLoadedData()
    {
        if (memory.userFacts == null)
            memory.userFacts = new List<FactItem>();
        if (memory.conversationSummaries == null)
            memory.conversationSummaries = new List<string>();
        if (memory.episodicMemory == null)
            memory.episodicMemory = new List<EpisodicItem>();
        if (memory.recentEmotions == null)
            memory.recentEmotions = new List<string>();
        if (memory.topicsDiscussed == null)
            memory.topicsDiscussed = new List<string>();

        var mergedFacts = new List<FactItem>();
        for (int i = 0; i < memory.userFacts.Count; i++)
        {
            var fact = memory.userFacts[i];
            if (string.IsNullOrWhiteSpace(fact.content))
                continue;

            fact.content = fact.content.Trim();
            fact.strength = Mathf.Max(1, fact.strength);
            if (string.IsNullOrWhiteSpace(fact.lastUpdated))
                fact.lastUpdated = DateTime.Now.ToString("yyyy-MM-dd");

            int existing = -1;
            string norm = NormalizeFact(fact.content);
            for (int j = 0; j < mergedFacts.Count; j++)
            {
                if (NormalizeFact(mergedFacts[j].content) == norm)
                {
                    existing = j;
                    break;
                }
            }

            if (existing >= 0)
            {
                FactItem baseItem = mergedFacts[existing];
                baseItem.strength = Mathf.Max(1, baseItem.strength) + fact.strength;
                DateTime baseDate = ParseIsoDate(baseItem.lastUpdated);
                DateTime nextDate = ParseIsoDate(fact.lastUpdated);
                if (nextDate > baseDate)
                    baseItem.lastUpdated = fact.lastUpdated;
                mergedFacts[existing] = baseItem;
            }
            else
            {
                mergedFacts.Add(fact);
            }
        }
        memory.userFacts = mergedFacts;

        var normalizedEpisodes = new List<EpisodicItem>();
        for (int i = 0; i < memory.episodicMemory.Count; i++)
        {
            var item = memory.episodicMemory[i];
            if (string.IsNullOrWhiteSpace(item.summary))
                continue;
            item.summary = item.summary.Trim();
            if (string.IsNullOrWhiteSpace(item.timestamp))
                item.timestamp = DateTime.Now.ToString("yyyy-MM-dd");
            item.tags = NormalizeTags(item.tags);
            normalizedEpisodes.Add(item);
        }
        memory.episodicMemory = normalizedEpisodes;
    }
}
