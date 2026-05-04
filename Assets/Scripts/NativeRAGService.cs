using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using UnityEngine;

/// <summary>
/// Lightweight native RAG implementation using TF-IDF + Cosine Similarity.
/// No external server or Python required. Fully self-contained in C#.
/// Optimized for RealAmadeus memory retrieval.
/// </summary>
public class NativeRAGService : MonoBehaviour
{
    public static NativeRAGService Instance { get; private set; }

    [Header("Settings")]
    [SerializeField] private int defaultTopK = 5;
    [SerializeField] private float similarityThreshold = 0.1f;
    [SerializeField] private float bm25K1 = 1.5f;
    [SerializeField] private float bm25B = 0.75f;

    private const string PREF_NATIVE_RAG_ENABLED = "Config_NativeRAG_Enabled";
    private const string PREF_NATIVE_RAG_TOPK = "Config_NativeRAG_TopK";
    private const string PREF_NATIVE_RAG_THRESHOLD = "Config_NativeRAG_Threshold";

    public bool IsEnabled
    {
        get => PlayerPrefs.GetInt(PREF_NATIVE_RAG_ENABLED, 0) == 1;
        set => PlayerPrefs.SetInt(PREF_NATIVE_RAG_ENABLED, value ? 1 : 0);
    }

    public int TopK
    {
        get => PlayerPrefs.GetInt(PREF_NATIVE_RAG_TOPK, defaultTopK);
        set => PlayerPrefs.SetInt(PREF_NATIVE_RAG_TOPK, value);
    }

    public float Threshold
    {
        get => PlayerPrefs.GetFloat(PREF_NATIVE_RAG_THRESHOLD, similarityThreshold);
        set => PlayerPrefs.SetFloat(PREF_NATIVE_RAG_THRESHOLD, value);
    }

    [Serializable]
    public class SearchResult
    {
        public string id;
        public string text;
        public float score;
    }

    [Serializable]
    private class Document
    {
        public string id;
        public string text;
        public int tokenCount;
        public Dictionary<string, float> rawTF; // raw term frequencies (not normalized)
        public Dictionary<string, float> tfidf; // legacy field, kept for compatibility
    }

    private List<Document> documents = new List<Document>();
    private Dictionary<string, int> documentFrequency = new Dictionary<string, int>();
    private int totalDocuments = 0;
    private HashSet<string> stopWords = new HashSet<string>();
    private bool needsRecompute = false;

    void Awake()
    {
        if (Instance != null && Instance != this)
        {
            Destroy(gameObject);
            return;
        }
        Instance = this;
        InitializeStopWords();
    }

    private void InitializeStopWords()
    {
        string[] stops = new string[]
        {
            "の","に","は","を","た","が","で","て","と","し","れ","さ","ある","いる","も","する","から","な","こと","として","い","や","れる","など","なっ","ー","せ","わ","が","か","け","ば","へ","まで","のみ","しか","たり","だり","なり","ず","ずつ","だの","なんか","なんて","なんど","どこ","どれ","どの","どんな","どう","どうして","どうぞ","どうも","どうにか","どうだ","どうだい","どうだん","どうしても","どうせ","どうにかこうにか","どうのこうの","どうみても",
            "the","is","at","which","on","a","an","and","or","but","in","of","to","for","with","as","by","from","that","this","it","be","are","was","were","been","have","has","had","do","does","did","will","would","could","should","may","might","can","shall","not","no","yes","so","if","then","than","too","very","just","now","only","also","its","his","her","him","she","he","they","them","their","we","us","our","you","your","i","me","my"
        };
        foreach (var s in stops) stopWords.Add(s);
    }

    /// <summary>
    /// Add or update a document in the index.
    /// </summary>
    public void AddDocument(string id, string text)
    {
        if (string.IsNullOrWhiteSpace(text)) return;
        documents.RemoveAll(d => d.id == id);

        var tokens = Tokenize(text);
        var rawTF = ComputeRawTF(tokens);

        var doc = new Document {
            id = id,
            text = text,
            tokenCount = tokens.Count,
            rawTF = rawTF,
            tfidf = ComputeTF(tokens) // kept for backward compatibility
        };
        documents.Add(doc);
        totalDocuments = documents.Count;

        foreach (var token in rawTF.Keys)
        {
            if (!documentFrequency.ContainsKey(token)) documentFrequency[token] = 0;
            documentFrequency[token]++;
        }
        needsRecompute = false; // BM25 does not need precomputed TF-IDF
    }

    /// <summary>
    /// Remove a document from the index.
    /// </summary>
    public void RemoveDocument(string id)
    {
        var doc = documents.Find(d => d.id == id);
        if (doc == null) return;
        documents.Remove(doc);
        totalDocuments = documents.Count;
        RebuildDF();
        needsRecompute = true;
    }

    /// <summary>
    /// Search for documents similar to the query using BM25 scoring.
    /// </summary>
    public List<SearchResult> Search(string query, int? topK = null)
    {
        if (!IsEnabled || documents.Count == 0) return new List<SearchResult>();

        var qTokens = Tokenize(query);
        if (qTokens.Count == 0) return new List<SearchResult>();

        float avgdl = ComputeAvgDocumentLength();

        var results = new List<SearchResult>();
        foreach (var doc in documents)
        {
            float score = ComputeBM25Score(qTokens, doc, avgdl);
            if (score >= Threshold)
            {
                results.Add(new SearchResult { id = doc.id, text = doc.text, score = score });
            }
        }

        int k = topK ?? TopK;
        return results.OrderByDescending(r => r.score).Take(k).ToList();
    }

    /// <summary>
    /// Get relevant context as a formatted string for LLM prompting.
    /// </summary>
    public string GetRelevantContext(string query, List<string> facts, int? topK = null)
    {
        if (!IsEnabled || facts == null || facts.Count == 0) return "";

        foreach (var fact in facts)
        {
            string id = fact.GetHashCode().ToString();
            if (!documents.Exists(d => d.id == id))
            {
                AddDocument(id, fact);
            }
        }

        var results = Search(query, topK);
        if (results.Count == 0) return "";

        System.Text.StringBuilder sb = new System.Text.StringBuilder();
        sb.AppendLine("【関連する記憶】");
        foreach (var r in results)
        {
            sb.AppendLine($"- {r.text}");
        }
        return sb.ToString();
    }

    /// <summary>
    /// Clear all indexed documents.
    /// </summary>
    public void Clear()
    {
        documents.Clear();
        documentFrequency.Clear();
        totalDocuments = 0;
        needsRecompute = false;
    }

    private void RebuildDF()
    {
        documentFrequency.Clear();
        foreach (var d in documents)
        {
            foreach (var token in d.tfidf.Keys.Distinct())
            {
                if (!documentFrequency.ContainsKey(token)) documentFrequency[token] = 0;
                documentFrequency[token]++;
            }
        }
    }

    private List<string> Tokenize(string text)
    {
        var tokens = new List<string>();
        if (string.IsNullOrWhiteSpace(text)) return tokens;

        var matches = Regex.Matches(text.ToLower(), @"[\u3040-\u309F]+|[\u30A0-\u30FF]+|[\u4E00-\u9FAF]+|[a-z0-9]+");
        foreach (Match m in matches)
        {
            string token = m.Value;
            if (token.Length >= 1 && !stopWords.Contains(token))
            {
                tokens.Add(token);
            }
        }
        return tokens;
    }

    private Dictionary<string, float> ComputeRawTF(List<string> tokens)
    {
        var tf = new Dictionary<string, float>();
        if (tokens.Count == 0) return tf;
        foreach (var token in tokens)
        {
            if (!tf.ContainsKey(token)) tf[token] = 0;
            tf[token]++;
        }
        return tf;
    }

    private Dictionary<string, float> ComputeTF(List<string> tokens)
    {
        var tf = ComputeRawTF(tokens);
        if (tokens.Count == 0) return tf;
        foreach (var key in tf.Keys.ToList())
        {
            tf[key] /= tokens.Count;
        }
        return tf;
    }

    private float ComputeAvgDocumentLength()
    {
        if (documents.Count == 0) return 0f;
        int total = 0;
        foreach (var doc in documents) total += doc.tokenCount;
        return (float)total / documents.Count;
    }

    private float ComputeBM25Score(List<string> qTokens, Document doc, float avgdl)
    {
        float score = 0f;
        var uniqueTokens = new HashSet<string>(qTokens);

        foreach (var token in uniqueTokens)
        {
            if (!documentFrequency.ContainsKey(token)) continue;

            int df = documentFrequency[token];
            float idf = Mathf.Log10((totalDocuments - df + 0.5f) / (df + 0.5f) + 1f);

            float f = doc.rawTF.ContainsKey(token) ? doc.rawTF[token] : 0f;
            float docLen = Mathf.Max(1f, doc.tokenCount);
            float denom = f + bm25K1 * (1f - bm25B + bm25B * (docLen / Mathf.Max(1f, avgdl)));
            if (denom <= 0f) continue;

            score += idf * (f * (bm25K1 + 1f)) / denom;
        }

        return score;
    }

    // Legacy TF-IDF methods kept for backward compatibility
    private void RecomputeAllTFIDF()
    {
        foreach (var doc in documents)
        {
            doc.tfidf = ComputeTFIDFVector(ComputeTF(Tokenize(doc.text)));
        }
        needsRecompute = false;
    }

    private Dictionary<string, float> ComputeTFIDFVector(Dictionary<string, float> tf)
    {
        var tfidf = new Dictionary<string, float>();
        foreach (var kvp in tf)
        {
            string token = kvp.Key;
            float tfVal = kvp.Value;
            int df = documentFrequency.ContainsKey(token) ? documentFrequency[token] : 1;
            float idf = Mathf.Log((float)totalDocuments / df) + 1f;
            tfidf[token] = tfVal * idf;
        }
        return tfidf;
    }

    private float CosineSimilarity(Dictionary<string, float> a, Dictionary<string, float> b)
    {
        float dot = 0f, normA = 0f, normB = 0f;
        foreach (var kvp in a)
        {
            normA += kvp.Value * kvp.Value;
            if (b.ContainsKey(kvp.Key))
                dot += kvp.Value * b[kvp.Key];
        }
        foreach (var kvp in b)
        {
            normB += kvp.Value * kvp.Value;
        }
        if (normA == 0f || normB == 0f) return 0f;
        return dot / (Mathf.Sqrt(normA) * Mathf.Sqrt(normB));
    }
}
