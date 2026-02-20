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
    public class KurisuMemory
    {
        public string userName = "";
        public List<string> userFacts = new List<string>();
        public List<string> conversationSummaries = new List<string>();
        public string lastSessionDate = "";
        public int totalInteractions = 0;
        public List<string> recentEmotions = new List<string>();
        public List<string> topicsDiscussed = new List<string>();
    }

    private KurisuMemory memory = new KurisuMemory();
    private string savePath;

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
            lines.Add("【ユーザーについて知っていること】");
            foreach (var fact in memory.userFacts)
                lines.Add($"- {fact}");
        }

        // Previous conversation summaries
        if (memory.conversationSummaries.Count > 0)
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
        if (string.IsNullOrEmpty(fact)) return;
        if (memory.userFacts.Contains(fact)) return;

        memory.userFacts.Add(fact);
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

        SaveMemory();
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
                Debug.Log($"[MemoryManager] Memory loaded. {memory.totalInteractions} total interactions.");

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
}
