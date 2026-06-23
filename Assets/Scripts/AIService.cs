using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Text;
using UnityEngine;
using UnityEngine.Networking;
using Debug = UnityEngine.Debug;

/// <summary>
/// Handles API communication with OpenAI, Gemini, Claude, and Groq providers.
/// Supports standard chat, streaming, and Groq Compound (web search).
/// Reads configuration from PlayerPrefs (set via ConfigPanel).
/// </summary>
public class AIService : MonoBehaviour
{
    private static bool IsAppForeground() => DesktopNotificationService.IsForegroundWindow();

    // PlayerPrefs keys (must match ConfigPanelController)
    private const string PREF_API_PROVIDER = "Config_ApiProvider";
    private const string PREF_API_KEY = "Config_ApiKey";
    private const string PREF_API_KEY_PREFIX = "Config_ApiKey_";
    private const string PREF_MODEL_NAME = "Config_ModelName";
    private const string PREF_MODEL_NAME_PREFIX = "Config_ModelName_";
    private const string PREF_WEB_SEARCH = "Config_WebSearch";
    // V1.3U: OpenAI-compatible custom base URL
    public const string PREF_OPENAI_COMPATIBLE = "Config_OpenAICompatible";
    public const string PREF_OPENAI_BASE_URL = "Config_OpenAIBaseUrl";

    // ─── V1.3U: in-flight cancellation ───
    private UnityWebRequest currentRequest;
    private Coroutine currentCoroutine;

    public bool IsRequestInFlight => currentRequest != null || currentCoroutine != null;

    public void CancelInFlightRequest()
    {
        try
        {
            if (currentCoroutine != null)
            {
                StopCoroutine(currentCoroutine);
                currentCoroutine = null;
            }
            if (currentRequest != null)
            {
                Debug.Log("[AIService] Cancelling in-flight request.");
                currentRequest.Abort();
                currentRequest.Dispose();
                currentRequest = null;
            }
        }
        catch (Exception ex)
        {
            Debug.LogWarning("[AIService] CancelInFlightRequest: " + ex.Message);
        }
    }

    private void OnDisable()
    {
        CancelInFlightRequest();
    }

    // Provider indices (matching ConfigPanel dropdown order)
    private const int PROVIDER_OPENAI = 0;
    private const int PROVIDER_GEMINI = 1;
    private const int PROVIDER_CLAUDE = 2;

    private const int PROVIDER_GROQ = 3;
    private const int PROVIDER_VERTEX = 4;
    private const int PROVIDER_OLLAMA = 5;
    private const int PROVIDER_OPENROUTER = 6;

    // ── Vertex AI Access Token Auto-Refresh (for gcloud) ──
    private string cachedVertexToken = null;
    private DateTime vertexTokenExpiry = DateTime.MinValue;
    private const int TOKEN_CACHE_MINUTES = 50; // refresh before 60min expiry

    [System.Serializable]
    public class ChatMessage
    {
        public string role; // "system", "user", "assistant"
        public string content;

        public ChatMessage(string role, string content)
        {
            this.role = role;
            this.content = content;
        }
    }

    /// <summary>
    /// Whether web search is enabled (uses Groq Compound).
    /// </summary>
    public bool IsWebSearchEnabled => PlayerPrefs.GetInt(PREF_WEB_SEARCH, 0) == 1;

    /// <summary>
    /// Sends a conversation to the configured API provider and returns the response.
    /// </summary>
    public void SendChat(List<ChatMessage> messages, Action<string> onSuccess, Action<string> onError)
    {
        int provider = PlayerPrefs.GetInt(PREF_API_PROVIDER, 0);
        const int MAX_PROVIDER = PROVIDER_OPENROUTER; // highest valid index
        if (provider < 0 || provider > MAX_PROVIDER)
        {
            Debug.LogWarning($"[AIService] 無効なプロバイダーインデックス {provider} を検出。0 (OpenAI) にリセットします。");
            provider = 0;
            PlayerPrefs.SetInt(PREF_API_PROVIDER, 0);
        }

        string apiKey = GetApiKeyForProvider(provider);
        
        // Try provider-specific model first, fallback to legacy
        string model = PlayerPrefs.GetString(PREF_MODEL_NAME_PREFIX + provider, "");
        if (string.IsNullOrEmpty(model)) model = PlayerPrefs.GetString(PREF_MODEL_NAME, "gpt-4o");

        // Vertex AI uses gcloud tokens, not API keys. OpenAI compatible mode and Ollama don't strictly require API keys.
        bool isCompat = (provider == PROVIDER_OPENAI && PlayerPrefs.GetInt(PREF_OPENAI_COMPATIBLE, 0) == 1);
        if (string.IsNullOrEmpty(apiKey) && provider != PROVIDER_VERTEX && provider != PROVIDER_OLLAMA && !isCompat)
        {
            onError?.Invoke("API Key が設定されていません。CONFIGから設定してください。");
            return;
        }

        switch (provider)
        {
            case PROVIDER_OPENAI:
                {
                    // V1.3U: when "Use Compatible API" is on, route to a custom base URL.
                    string openAIUrl = null;
                    if (PlayerPrefs.GetInt(PREF_OPENAI_COMPATIBLE, 0) == 1)
                    {
                        string baseUrl = PlayerPrefs.GetString(PREF_OPENAI_BASE_URL, "").Trim();
                        if (!string.IsNullOrEmpty(baseUrl))
                        {
                            openAIUrl = baseUrl.TrimEnd('/') + "/chat/completions";
                        }
                    }
                    currentCoroutine = StartCoroutine(SendOpenAI(apiKey, model, messages, onSuccess, onError, openAIUrl, false, true));
                }
                break;
            case PROVIDER_GEMINI:
                currentCoroutine = StartCoroutine(SendGemini(apiKey, model, messages, onSuccess, onError));
                break;
            case PROVIDER_CLAUDE:
                currentCoroutine = StartCoroutine(SendClaude(apiKey, model, messages, onSuccess, onError));
                break;
            case PROVIDER_GROQ:
                if (IsWebSearchEnabled)
                    currentCoroutine = StartCoroutine(SendGroqCompound(apiKey, messages, onSuccess, onError));
                else
                    currentCoroutine = StartCoroutine(SendGroq(apiKey, model, messages, onSuccess, onError));
                break;
            case PROVIDER_VERTEX:
                string projectId = PlayerPrefs.GetString("Config_VertexProject", "");
                string location = PlayerPrefs.GetString("Config_VertexLocation", "us-central1");

#if UNITY_WEBGL && !UNITY_EDITOR
                onError?.Invoke(LocalizationManager.GetOrCreate().T("api_error_webgl_vertex"));
#else
                currentCoroutine = StartCoroutine(GetVertexAccessTokenGcloudAsync((vertexToken) =>
                {
                    if (string.IsNullOrEmpty(vertexToken))
                    {
                        onError?.Invoke(LocalizationManager.GetOrCreate().T("api_error_vertex_token"));
                        return;
                    }
                    currentCoroutine = StartCoroutine(SendVertexAI(vertexToken, projectId, location, model, messages, onSuccess, onError));
                }));
#endif
                break;
            case PROVIDER_OLLAMA:
                string ollamaHost = PlayerPrefs.GetString("Config_OllamaHost", "http://localhost:11434");
                if (string.IsNullOrEmpty(ollamaHost)) ollamaHost = "http://localhost:11434";
                string ollamaUrl = ollamaHost.TrimEnd('/') + "/v1/chat/completions";
                currentCoroutine = StartCoroutine(SendOpenAI(apiKey, model, messages, onSuccess, onError, ollamaUrl));
                break;
            case PROVIDER_OPENROUTER:
                const string openrouterUrl = "https://openrouter.ai/api/v1/chat/completions";
                currentCoroutine = StartCoroutine(SendOpenAI(apiKey, model, messages, onSuccess, onError, openrouterUrl, true));
                break;
            default:
                onError?.Invoke($"Unknown provider index: {provider}");
                break;
        }
    }

    /// <summary>
    /// Sends a streaming chat request (Groq only). Calls onToken for each token chunk,
    /// then onComplete when finished.
    /// </summary>
    public void SendChatStreaming(List<ChatMessage> messages, Action<string> onToken, Action<string> onComplete, Action<string> onError)
    {
        int provider = PlayerPrefs.GetInt(PREF_API_PROVIDER, 0);
        const int MAX_PROVIDER_S = PROVIDER_OPENROUTER; // highest valid index
        if (provider < 0 || provider > MAX_PROVIDER_S)
        {
            Debug.LogWarning($"[AIService] 無効なプロバイダーインデックス {provider} を検出。0 (OpenAI) にリセットします。");
            provider = 0;
            PlayerPrefs.SetInt(PREF_API_PROVIDER, 0);
        }

        string apiKey = GetApiKeyForProvider(provider);
        
        // Try provider-specific model first, fallback to legacy
        string model = PlayerPrefs.GetString(PREF_MODEL_NAME_PREFIX + provider, "");
        if (string.IsNullOrEmpty(model)) model = PlayerPrefs.GetString(PREF_MODEL_NAME, "gpt-4o");

        // Vertex AI uses gcloud tokens, not API keys. OpenAI compatible mode and Ollama don't strictly require API keys.
        bool isCompat = (provider == PROVIDER_OPENAI && PlayerPrefs.GetInt(PREF_OPENAI_COMPATIBLE, 0) == 1);
        if (string.IsNullOrEmpty(apiKey) && provider != PROVIDER_VERTEX && provider != PROVIDER_OLLAMA && !isCompat)
        {
            onError?.Invoke("API Key が設定されていません。CONFIGから設定してください。");
            return;
        }

        if (provider == PROVIDER_GROQ)
        {
            if (IsWebSearchEnabled)
                currentCoroutine = StartCoroutine(SendGroqCompoundStreaming(apiKey, messages, onToken, onComplete, onError));
            else
                currentCoroutine = StartCoroutine(SendGroqStreaming(apiKey, model, messages, onToken, onComplete, onError));
        }
        else if (provider == PROVIDER_VERTEX)
        {
            string projectId = PlayerPrefs.GetString("Config_VertexProject", "");
            string location = PlayerPrefs.GetString("Config_VertexLocation", "us-central1");

#if UNITY_WEBGL && !UNITY_EDITOR
            onError?.Invoke(LocalizationManager.GetOrCreate().T("api_error_webgl_vertex"));
#else
            currentCoroutine = StartCoroutine(GetVertexAccessTokenGcloudAsync((vertexTokenStream) =>
            {
                if (string.IsNullOrEmpty(vertexTokenStream))
                {
                    onError?.Invoke(LocalizationManager.GetOrCreate().T("api_error_vertex_token"));
                    return;
                }
                currentCoroutine = StartCoroutine(SendVertexAIStreaming(vertexTokenStream, projectId, location, model, messages, onToken, onComplete, onError));
            }));
#endif
        }
        else if (provider == PROVIDER_OLLAMA)
        {
            string ollamaHost = PlayerPrefs.GetString("Config_OllamaHost", "http://localhost:11434");
            if (string.IsNullOrEmpty(ollamaHost)) ollamaHost = "http://localhost:11434";
            string ollamaUrl = ollamaHost.TrimEnd('/') + "/v1/chat/completions";
            currentCoroutine = StartCoroutine(SendOpenAIStreaming(apiKey, model, messages, onToken, onComplete, onError, ollamaUrl, false));
        }
        else if (provider == PROVIDER_OPENROUTER)
        {
            const string openrouterUrl = "https://openrouter.ai/api/v1/chat/completions";
            currentCoroutine = StartCoroutine(SendOpenAIStreaming(apiKey, model, messages, onToken, onComplete, onError, openrouterUrl, true));
        }
        else if (provider == PROVIDER_OPENAI && PlayerPrefs.GetInt(PREF_OPENAI_COMPATIBLE, 0) == 1)
        {
            // V1.3U: when OpenAI-compatible mode is on, stream from the user's custom base URL.
            string baseUrl = PlayerPrefs.GetString(PREF_OPENAI_BASE_URL, "").Trim();
            string compatibleUrl = null;
            if (!string.IsNullOrEmpty(baseUrl))
            {
                compatibleUrl = baseUrl.TrimEnd('/') + "/chat/completions";
            }
            currentCoroutine = StartCoroutine(SendOpenAIStreaming(apiKey, model, messages, onToken, onComplete, onError, compatibleUrl, false));
        }
        else
        {
            // Fallback: use non-streaming for other providers (OpenAI without compatible mode, Gemini, Claude)
            SendChat(messages, s => { onToken(s); onComplete(s); }, onError);
        }
    }

    // ─────────────────────────────────────────
    // OpenAI (ChatCompletion API)
    // ─────────────────────────────────────────
    private IEnumerator SendOpenAI(string apiKey, string model, List<ChatMessage> messages, Action<string> onSuccess, Action<string> onError, string urlOverride = null, bool isOpenRouter = false, bool isOpenAICompatible = false)
    {
        string url = string.IsNullOrEmpty(urlOverride) ? "https://api.openai.com/v1/chat/completions" : urlOverride;

        string messagesJson = BuildOpenAIMessages(messages);
        string body;
        if (PlayerPrefs.GetInt(ConfigPanelController.PREF_TOOLS_ENABLED, 0) == 1 && !isOpenAICompatible)
        {
            string toolsJson = "[{\"type\":\"function\",\"function\":{\"name\":\"get_current_time\",\"description\":\"Get the current time to answer questions about what time it is.\",\"parameters\":{\"type\":\"object\",\"properties\":{},\"required\":[]}}}]";
            body = $"{{\"model\":\"{EscapeJson(model)}\",\"messages\":{messagesJson},\"max_tokens\":2048,\"tools\":{toolsJson}}}";
        }
        else
        {
            body = $"{{\"model\":\"{EscapeJson(model)}\",\"messages\":{messagesJson},\"max_tokens\":2048}}";
        }

        using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
        {
            currentRequest = req;
            byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
            req.uploadHandler = new UploadHandlerRaw(bodyRaw);
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            if (!string.IsNullOrEmpty(apiKey))
            {
                req.SetRequestHeader("Authorization", $"Bearer {apiKey}");
            }

            if (isOpenRouter)
            {
                req.SetRequestHeader("HTTP-Referer", "https://realamadeus.ai");
                req.SetRequestHeader("X-Title", "RealAmadeus");
            }

            req.timeout = 60;

            yield return req.SendWebRequest();
            currentRequest = null;

            if (req.result != UnityWebRequest.Result.Success)
            {
                if (req.result == UnityWebRequest.Result.ConnectionError && req.error == "Request aborted")
                {
                    Debug.Log("[AIService] Request aborted (likely cancelled).");
                    yield break;
                }
                onError?.Invoke($"OpenAI Error: {req.error}\n{req.downloadHandler.text}");
            }
            else
            {
                string response = ExtractOpenAIResponse(req.downloadHandler.text);
                onSuccess?.Invoke(response);
            }
        }
    }

    private string GetApiKeyForProvider(int provider)
    {
        string providerKey = SecurePrefs.GetProtectedString(
            PREF_API_KEY_PREFIX + provider,
            PlayerPrefs.GetString(PREF_API_KEY_PREFIX + provider, ""));
        if (!string.IsNullOrEmpty(providerKey))
        {
            return providerKey;
        }

        return SecurePrefs.GetProtectedString(
            PREF_API_KEY,
            PlayerPrefs.GetString(PREF_API_KEY, ""));
    }

    private string BuildOpenAIMessages(List<ChatMessage> messages)
    {
        StringBuilder sb = new StringBuilder("[");
        for (int i = 0; i < messages.Count; i++)
        {
            if (i > 0) sb.Append(",");
            sb.Append($"{{\"role\":\"{messages[i].role}\",\"content\":\"{EscapeJson(messages[i].content)}\"}}");
        }
        sb.Append("]");
        return sb.ToString();
    }

    private string ExtractOpenAIResponse(string json)
    {
        try
        {
            if (json.Contains("\"tool_calls\"") && json.Contains("\"name\": \"get_current_time\""))
            {
                return $"[System: Tool called: get_current_time. Current time is {DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss")}. Assistant should incorporate this.]\n";
            }
            int contentIdx = json.IndexOf("\"content\":", json.IndexOf("\"message\""));
            if (contentIdx < 0) return "[Parse Error]";
            int start = json.IndexOf("\"", contentIdx + 10) + 1;
            int end = FindClosingQuote(json, start);
            string content = UnescapeJson(json.Substring(start, end - start));
            if (string.IsNullOrEmpty(content) || content == "null")
            {
                int toolsIdx = json.IndexOf("\"tool_calls\"");
                if (toolsIdx > 0)
                {
                    return $"[System: Tool called: get_current_time. Current time is {DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss")}. Assistant should incorporate this.]\n";
                }
            }
            return content;
        }
        catch (Exception e)
        {
            Debug.LogError($"OpenAI parse error: {e.Message}\nRaw: {json}");
            return "[Parse Error]";
        }
    }

    // ─────────────────────────────────────────
    // Gemini (Google AI)
    // ─────────────────────────────────────────
    private IEnumerator SendGemini(string apiKey, string model, List<ChatMessage> messages, Action<string> onSuccess, Action<string> onError)
    {
        if (string.IsNullOrEmpty(model) || model.StartsWith("gpt")) model = "gemini-2.0-flash";
        string url = $"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={apiKey}";

        string body = BuildGeminiBody(messages);

        using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
        {
            byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
            req.uploadHandler = new UploadHandlerRaw(bodyRaw);
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            req.timeout = 60;

            yield return req.SendWebRequest();

            if (req.result != UnityWebRequest.Result.Success)
            {
                onError?.Invoke($"Gemini Error: {req.error}\n{req.downloadHandler.text}");
            }
            else
            {
                string response = ExtractGeminiResponse(req.downloadHandler.text);
                onSuccess?.Invoke(response);
            }
        }
    }

    private string BuildGeminiBody(List<ChatMessage> messages, bool useGrounding = false)
    {
        StringBuilder contents = new StringBuilder();
        string systemInstruction = "";
        bool first = true;

        foreach (var msg in messages)
        {
            if (msg.role == "system")
            {
                systemInstruction = msg.content;
                continue;
            }

            if (!first) contents.Append(",");
            first = false;

            string geminiRole = msg.role == "assistant" ? "model" : "user";
            contents.Append($"{{\"role\":\"{geminiRole}\",\"parts\":[{{\"text\":\"{EscapeJson(msg.content)}\"}}]}}");
        }

        StringBuilder body = new StringBuilder("{");
        if (!string.IsNullOrEmpty(systemInstruction))
        {
            body.Append($"\"system_instruction\":{{\"parts\":[{{\"text\":\"{EscapeJson(systemInstruction)}\"}}]}},");
        }
        body.Append($"\"contents\":[{contents}]");
        body.Append(",\"generationConfig\":{\"maxOutputTokens\":2048}");
        // Google Search grounding (Vertex AI)
        if (useGrounding)
        {
            body.Append(",\"tools\":[{\"googleSearch\":{}}]");
        }
        body.Append("}");
        return body.ToString();
    }

    private string ExtractGeminiResponse(string json)
    {
        try
        {
            int textIdx = json.IndexOf("\"text\":");
            if (textIdx < 0) return "[Parse Error]";
            int start = json.IndexOf("\"", textIdx + 6) + 1;
            int end = FindClosingQuote(json, start);
            return UnescapeJson(json.Substring(start, end - start));
        }
        catch (Exception e)
        {
            Debug.LogError($"Gemini parse error: {e.Message}\nRaw: {json}");
            return "[Parse Error]";
        }
    }

    // ─────────────────────────────────────────
    // Claude (Anthropic Messages API)
    // ─────────────────────────────────────────
    private IEnumerator SendClaude(string apiKey, string model, List<ChatMessage> messages, Action<string> onSuccess, Action<string> onError)
    {
        if (string.IsNullOrEmpty(model) || model.StartsWith("gpt") || model.StartsWith("gemini")) model = "claude-3-7-sonnet-20250219";
        string url = "https://api.anthropic.com/v1/messages";

        string body = BuildClaudeBody(model, messages);

        using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
        {
            byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
            req.uploadHandler = new UploadHandlerRaw(bodyRaw);
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            req.SetRequestHeader("x-api-key", apiKey);
            req.SetRequestHeader("anthropic-version", "2023-06-01");
            req.timeout = 60;

            yield return req.SendWebRequest();

            if (req.result != UnityWebRequest.Result.Success)
            {
                onError?.Invoke($"Claude Error: {req.error}\n{req.downloadHandler.text}");
            }
            else
            {
                string response = ExtractClaudeResponse(req.downloadHandler.text);
                onSuccess?.Invoke(response);
            }
        }
    }

    private string BuildClaudeBody(string model, List<ChatMessage> messages)
    {
        string systemText = "";
        StringBuilder msgArray = new StringBuilder("[");
        bool first = true;

        foreach (var msg in messages)
        {
            if (msg.role == "system")
            {
                systemText = msg.content;
                continue;
            }

            if (!first) msgArray.Append(",");
            first = false;
            msgArray.Append($"{{\"role\":\"{msg.role}\",\"content\":\"{EscapeJson(msg.content)}\"}}");
        }
        msgArray.Append("]");

        StringBuilder body = new StringBuilder("{");
        body.Append($"\"model\":\"{EscapeJson(model)}\",");
        body.Append("\"max_tokens\":2048,");
        if (!string.IsNullOrEmpty(systemText))
        {
            body.Append($"\"system\":\"{EscapeJson(systemText)}\",");
        }
        body.Append($"\"messages\":{msgArray}");
        body.Append("}");
        return body.ToString();
    }

    private string ExtractClaudeResponse(string json)
    {
        try
        {
            int textIdx = json.IndexOf("\"text\":", json.IndexOf("\"content\""));
            if (textIdx < 0) return "[Parse Error]";
            int start = json.IndexOf("\"", textIdx + 6) + 1;
            int end = FindClosingQuote(json, start);
            return UnescapeJson(json.Substring(start, end - start));
        }
        catch (Exception e)
        {
            Debug.LogError($"Claude parse error: {e.Message}\nRaw: {json}");
            return "[Parse Error]";
        }
    }

    // ─────────────────────────────────────────
    // Groq (OpenAI-compatible API)
    // ─────────────────────────────────────────
    private IEnumerator SendGroq(string apiKey, string model, List<ChatMessage> messages, Action<string> onSuccess, Action<string> onError)
    {
        if (string.IsNullOrEmpty(model) || (!model.Contains("llama") && !model.Contains("mixtral") && !model.Contains("gemma") && !model.Contains("qwen") && !model.Contains("deepseek") && !model.Contains("compound")))
            model = "qwen3-32b";
        string url = "https://api.groq.com/openai/v1/chat/completions";

        string messagesJson = BuildOpenAIMessages(messages);
        string reasoningParam = model.Contains("qwen") ? ",\"reasoning_format\":\"hidden\"" : "";
        string body = $"{{\"model\":\"{EscapeJson(model)}\",\"messages\":{messagesJson},\"max_tokens\":2048,\"temperature\":0.85,\"top_p\":0.9{reasoningParam}}}";

        using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
        {
            byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
            req.uploadHandler = new UploadHandlerRaw(bodyRaw);
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            req.SetRequestHeader("Authorization", $"Bearer {apiKey}");
            req.timeout = 60;

            yield return req.SendWebRequest();

            if (req.result != UnityWebRequest.Result.Success)
            {
                onError?.Invoke($"Groq Error: {req.error}\n{req.downloadHandler.text}");
            }
            else
            {
                string response = ExtractOpenAIResponse(req.downloadHandler.text);
                onSuccess?.Invoke(response);
            }
        }
    }

    // ─────────────────────────────────────────
    // Groq Streaming (SSE)
    // ─────────────────────────────────────────
    private IEnumerator SendGroqStreaming(string apiKey, string model, List<ChatMessage> messages, Action<string> onToken, Action<string> onComplete, Action<string> onError)
    {
        if (string.IsNullOrEmpty(model) || (!model.Contains("llama") && !model.Contains("mixtral") && !model.Contains("gemma") && !model.Contains("qwen") && !model.Contains("deepseek")))
            model = "qwen3-32b";
        string url = "https://api.groq.com/openai/v1/chat/completions";

        string messagesJson = BuildOpenAIMessages(messages);
        string reasoningParam = model.Contains("qwen") ? ",\"reasoning_format\":\"hidden\"" : "";
        string body = $"{{\"model\":\"{EscapeJson(model)}\",\"messages\":{messagesJson},\"max_tokens\":2048,\"temperature\":0.85,\"top_p\":0.9,\"stream\":true{reasoningParam}}}";

        using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
        {
            byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
            req.uploadHandler = new UploadHandlerRaw(bodyRaw);
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            req.SetRequestHeader("Authorization", $"Bearer {apiKey}");
            req.SetRequestHeader("Accept", "text/event-stream");
            req.timeout = 120;

            var op = req.SendWebRequest();
            StringBuilder fullResponse = new StringBuilder();
            int lastProcessedIndex = 0;

            // Poll for streaming data
            while (!op.isDone)
            {
                if (IsAppForeground() && req.downloadHandler != null)
                {
                    string currentData = req.downloadHandler.text;
                    if (currentData.Length > lastProcessedIndex)
                    {
                        string newData = currentData.Substring(lastProcessedIndex);
                        lastProcessedIndex = currentData.Length;

                        // Parse SSE lines
                        string[] lines = newData.Split('\n');
                        foreach (string line in lines)
                        {
                            if (line.StartsWith("data: "))
                            {
                                string jsonChunk = line.Substring(6).Trim();
                                if (jsonChunk == "[DONE]") continue;

                                string token = ExtractStreamToken(jsonChunk);
                                if (!string.IsNullOrEmpty(token))
                                {
                                    fullResponse.Append(token);
                                    onToken?.Invoke(token);
                                }
                            }
                        }
                    }
                }
                if (IsAppForeground()) yield return null;
                else yield return new WaitForSecondsRealtime(0.05f);
            }

            // Process any remaining data
            if (req.downloadHandler != null)
            {
                string finalData = req.downloadHandler.text;
                if (finalData.Length > lastProcessedIndex)
                {
                    string remaining = finalData.Substring(lastProcessedIndex);
                    string[] lines = remaining.Split('\n');
                    foreach (string line in lines)
                    {
                        if (line.StartsWith("data: "))
                        {
                            string jsonChunk = line.Substring(6).Trim();
                            if (jsonChunk == "[DONE]") continue;

                            string token = ExtractStreamToken(jsonChunk);
                            if (!string.IsNullOrEmpty(token))
                            {
                                fullResponse.Append(token);
                                onToken?.Invoke(token);
                            }
                        }
                    }
                }
            }

            if (req.result != UnityWebRequest.Result.Success)
            {
                onError?.Invoke($"Groq Streaming Error: {req.error}\n{req.downloadHandler?.text}");
            }
            else
            {
                onComplete?.Invoke(fullResponse.ToString());
            }
        }
    }

    /// <summary>
    /// Extract a token from a streaming JSON chunk.
    /// Format: {"choices":[{"delta":{"content":"token"}}]}
    /// </summary>
    private string ExtractStreamToken(string json)
    {
        try
        {
            int deltaIdx = json.IndexOf("\"delta\"");
            if (deltaIdx < 0) return null;

            int contentIdx = json.IndexOf("\"content\":", deltaIdx);
            if (contentIdx < 0) return null;

            int start = json.IndexOf("\"", contentIdx + 10) + 1;
            if (start <= 0) return null;

            int end = FindClosingQuote(json, start);
            string token = UnescapeJson(json.Substring(start, end - start));
            return token;
        }
        catch
        {
            return null;
        }
    }

    // ─────────────────────────────────────────
    // OpenAI-compatible Streaming (for OpenRouter, Ollama)
    // ─────────────────────────────────────────
    private IEnumerator SendOpenAIStreaming(string apiKey, string model, List<ChatMessage> messages, Action<string> onToken, Action<string> onComplete, Action<string> onError, string urlOverride = null, bool isOpenRouter = false)
    {
        string url = string.IsNullOrEmpty(urlOverride) ? "https://api.openai.com/v1/chat/completions" : urlOverride;

        string messagesJson = BuildOpenAIMessages(messages);
        string body = $"{{\"model\":\"{EscapeJson(model)}\",\"messages\":{messagesJson},\"max_tokens\":2048,\"stream\":true}}";

        using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
        {
            currentRequest = req;
            byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
            req.uploadHandler = new UploadHandlerRaw(bodyRaw);
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            if (!string.IsNullOrEmpty(apiKey))
            {
                req.SetRequestHeader("Authorization", $"Bearer {apiKey}");
            }
            req.SetRequestHeader("Accept", "text/event-stream");
            req.timeout = 120;

            if (isOpenRouter)
            {
                req.SetRequestHeader("HTTP-Referer", "https://realamadeus.ai");
                req.SetRequestHeader("X-Title", "RealAmadeus");
            }

            var op = req.SendWebRequest();
            StringBuilder fullResponse = new StringBuilder();
            int lastProcessedIndex = 0;

            while (!op.isDone)
            {
                if (IsAppForeground() && req.downloadHandler != null)
                {
                    string currentData = req.downloadHandler.text;
                    if (currentData.Length > lastProcessedIndex)
                    {
                        string newData = currentData.Substring(lastProcessedIndex);
                        lastProcessedIndex = currentData.Length;

                        string[] lines = newData.Split('\n');
                        foreach (string line in lines)
                        {
                            if (line.StartsWith("data: "))
                            {
                                string jsonChunk = line.Substring(6).Trim();
                                if (jsonChunk == "[DONE]") continue;

                                string token = ExtractStreamToken(jsonChunk);
                                if (!string.IsNullOrEmpty(token))
                                {
                                    fullResponse.Append(token);
                                    onToken?.Invoke(token);
                                }
                            }
                        }
                    }
                }
                if (IsAppForeground()) yield return null;
                else yield return new WaitForSecondsRealtime(0.05f);
            }
            currentRequest = null;

            if (req.result != UnityWebRequest.Result.Success)
            {
                if (req.result == UnityWebRequest.Result.ConnectionError && req.error == "Request aborted")
                {
                    Debug.Log("[AIService] Streaming request aborted (likely cancelled).");
                    yield break;
                }
                onError?.Invoke($"OpenAI Streaming Error: {req.error}\n{req.downloadHandler?.text}");
            }
            else
            {
                onComplete?.Invoke(fullResponse.ToString());
            }
        }
    }

    // ─────────────────────────────────────────
    // Groq Compound (Web Search)
    // ─────────────────────────────────────────・・
    private IEnumerator SendGroqCompound(string apiKey, List<ChatMessage> messages, Action<string> onSuccess, Action<string> onError)
    {
        string url = "https://api.groq.com/openai/v1/chat/completions";
        string model = "groq/compound";

        string messagesJson = BuildOpenAIMessages(messages);
        string body = $"{{\"model\":\"{model}\",\"messages\":{messagesJson},\"max_tokens\":2048,\"temperature\":0.85}}";

        using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
        {
            byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
            req.uploadHandler = new UploadHandlerRaw(bodyRaw);
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            req.SetRequestHeader("Authorization", $"Bearer {apiKey}");
            req.timeout = 60;

            yield return req.SendWebRequest();

            if (req.result != UnityWebRequest.Result.Success)
            {
                onError?.Invoke($"Groq Compound Error: {req.error}\n{req.downloadHandler.text}");
            }
            else
            {
                string response = ExtractOpenAIResponse(req.downloadHandler.text);
                onSuccess?.Invoke(response);
            }
        }
    }

    private IEnumerator SendGroqCompoundStreaming(string apiKey, List<ChatMessage> messages, Action<string> onToken, Action<string> onComplete, Action<string> onError)
    {
        string url = "https://api.groq.com/openai/v1/chat/completions";
        string model = "groq/compound";

        string messagesJson = BuildOpenAIMessages(messages);
        string body = $"{{\"model\":\"{model}\",\"messages\":{messagesJson},\"max_tokens\":2048,\"temperature\":0.85,\"stream\":true}}";

        using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
        {
            byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
            req.uploadHandler = new UploadHandlerRaw(bodyRaw);
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            req.SetRequestHeader("Authorization", $"Bearer {apiKey}");
            req.SetRequestHeader("Accept", "text/event-stream");
            req.timeout = 120;

            var op = req.SendWebRequest();
            StringBuilder fullResponse = new StringBuilder();
            int lastProcessedIndex = 0;

            while (!op.isDone)
            {
                if (IsAppForeground() && req.downloadHandler != null)
                {
                    string currentData = req.downloadHandler.text;
                    if (currentData.Length > lastProcessedIndex)
                    {
                        string newData = currentData.Substring(lastProcessedIndex);
                        lastProcessedIndex = currentData.Length;

                        string[] lines = newData.Split('\n');
                        foreach (string line in lines)
                        {
                            if (line.StartsWith("data: "))
                            {
                                string jsonChunk = line.Substring(6).Trim();
                                if (jsonChunk == "[DONE]") continue;

                                string token = ExtractStreamToken(jsonChunk);
                                if (!string.IsNullOrEmpty(token))
                                {
                                    fullResponse.Append(token);
                                    onToken?.Invoke(token);
                                }
                            }
                        }
                    }
                }
                if (IsAppForeground()) yield return null;
                else yield return new WaitForSecondsRealtime(0.05f);
            }

            // Process remaining
            if (req.downloadHandler != null)
            {
                string finalData = req.downloadHandler.text;
                if (finalData.Length > lastProcessedIndex)
                {
                    string remaining = finalData.Substring(lastProcessedIndex);
                    string[] lines = remaining.Split('\n');
                    foreach (string line in lines)
                    {
                        if (line.StartsWith("data: "))
                        {
                            string jsonChunk = line.Substring(6).Trim();
                            if (jsonChunk == "[DONE]") continue;

                            string token = ExtractStreamToken(jsonChunk);
                            if (!string.IsNullOrEmpty(token))
                            {
                                fullResponse.Append(token);
                                onToken?.Invoke(token);
                            }
                        }
                    }
                }
            }

            if (req.result != UnityWebRequest.Result.Success)
            {
                onError?.Invoke($"Groq Compound Streaming Error: {req.error}\n{req.downloadHandler?.text}");
            }
            else
            {
                onComplete?.Invoke(fullResponse.ToString());
            }
        }
    }

    // ─────────────────────────────────────────
    // JSON Utilities
    // ─────────────────────────────────────────
    private static string EscapeJson(string s)
    {
        if (string.IsNullOrEmpty(s)) return "";
        return s.Replace("\\", "\\\\")
                .Replace("\"", "\\\"")
                .Replace("\n", "\\n")
                .Replace("\r", "\\r")
                .Replace("\t", "\\t");
    }

    private static string UnescapeJson(string s)
    {
        if (string.IsNullOrEmpty(s)) return "";
        return s.Replace("\\n", "\n")
                .Replace("\\r", "\r")
                .Replace("\\t", "\t")
                .Replace("\\\"", "\"")
                .Replace("\\\\", "\\");
    }

    /// <summary>
    /// Finds the closing quote that isn't escaped.
    /// </summary>
    private static int FindClosingQuote(string json, int startAfterQuote)
    {
        for (int i = startAfterQuote; i < json.Length; i++)
        {
            if (json[i] == '"')
            {
                int backslashes = 0;
                int j = i - 1;
                while (j >= 0 && json[j] == '\\') { backslashes++; j--; }
                if (backslashes % 2 == 0) return i;
            }
        }
        return json.Length;
    }

    // ─────────────────────────────────────────
    // Vertex AI Methods
    // ─────────────────────────────────────────

    private IEnumerator GetVertexAccessTokenGcloudAsync(System.Action<string> onResult)
    {
        // Return cached token if still valid
        if (!string.IsNullOrEmpty(cachedVertexToken) && DateTime.Now < vertexTokenExpiry)
        {
            onResult?.Invoke(cachedVertexToken);
            yield break;
        }

        string resultToken = null;
        bool taskDone = false;

        // Run gcloud process on a background thread to avoid freezing the main thread
        System.Threading.Tasks.Task.Run(() =>
        {
            try
            {
                string gcloudPath = "gcloud";
                #if UNITY_EDITOR_WIN || UNITY_STANDALONE_WIN
                string[] possiblePaths = new string[]
                {
                    "gcloud",
                    System.IO.Path.Combine(System.Environment.GetFolderPath(System.Environment.SpecialFolder.LocalApplicationData), "Google", "Cloud SDK", "google-cloud-sdk", "bin", "gcloud.cmd"),
                    System.IO.Path.Combine(System.Environment.GetFolderPath(System.Environment.SpecialFolder.ProgramFiles), "Google", "Cloud SDK", "google-cloud-sdk", "bin", "gcloud.cmd"),
                    @"C:\Users\" + System.Environment.UserName + @"\AppData\Local\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd"
                };
                #else
                string[] possiblePaths = new string[] { "gcloud", "/usr/local/bin/gcloud", "/usr/bin/gcloud" };
                #endif

                foreach (string path in possiblePaths)
                {
                    string token = RunGcloudCommandSync(path);
                    if (!string.IsNullOrEmpty(token))
                    {
                        resultToken = token;
                        break;
                    }
                }
            }
            catch (System.Exception e)
            {
                Debug.LogWarning($"[Vertex AI] gcloud トークン自動取得失敗: {e.Message}");
            }
            finally
            {
                taskDone = true;
            }
        });

        // Wait for background thread to finish (non-blocking main thread)
        while (!taskDone)
        {
            yield return null;
        }

        if (!string.IsNullOrEmpty(resultToken))
        {
            cachedVertexToken = resultToken;
            vertexTokenExpiry = DateTime.Now.AddMinutes(TOKEN_CACHE_MINUTES);
            Debug.Log($"[Vertex AI] アクセストークンを自動取得しました (gcloud) 有効期限: {TOKEN_CACHE_MINUTES}分");
            onResult?.Invoke(cachedVertexToken);
            yield break;
        }

        // Fallback to manual API key
        int currentProvider = PlayerPrefs.GetInt(PREF_API_PROVIDER, 0);
        string manualKey = PlayerPrefs.GetString(PREF_API_KEY_PREFIX + currentProvider, "");
        if (string.IsNullOrEmpty(manualKey)) manualKey = PlayerPrefs.GetString(PREF_API_KEY, "");

        onResult?.Invoke(!string.IsNullOrEmpty(manualKey) ? manualKey : null);
    }

    private string RunGcloudCommandSync(string gcloudPath)
    {
        try
        {
            var process = new System.Diagnostics.Process();
            process.StartInfo.FileName = gcloudPath;
            process.StartInfo.Arguments = "auth print-access-token";
            process.StartInfo.UseShellExecute = false;
            process.StartInfo.RedirectStandardOutput = true;
            process.StartInfo.RedirectStandardError = true;
            process.StartInfo.CreateNoWindow = true;
            process.StartInfo.WindowStyle = System.Diagnostics.ProcessWindowStyle.Hidden;

            process.Start();
            string output = process.StandardOutput.ReadToEnd().Trim();
            string error = process.StandardError.ReadToEnd().Trim();
            process.WaitForExit(10000);

            if (process.ExitCode == 0 && !string.IsNullOrEmpty(output) && output.StartsWith("ya29."))
            {
                return output;
            }
        }
        catch { }
        return null;
    }


    // ─────────────────────────────────────────
    // Vertex AI (Google Cloud)
    // ─────────────────────────────────────────
    // Multi-region failover configuration
    private readonly string[] VERTEX_REGIONS = 
    { 
        "us-central1", 
        "us-west1", 
        "us-east4", 
        "asia-northeast1", 
        "northamerica-northeast1" 
    };

    /// <summary>
    /// Returns a list of regions to try, starting with the preferred location.
    /// Deduplicates and ensures coverage across available regions.
    /// </summary>
    private List<string> GetTargetRegions(string preferredLocation)
    {
        List<string> targets = new List<string>();
        if (!string.IsNullOrEmpty(preferredLocation)) 
        {
            targets.Add(preferredLocation);
        }

        foreach (var region in VERTEX_REGIONS)
        {
            if (!targets.Contains(region))
            {
                targets.Add(region);
            }
        }
        return targets;
    }

    /// <summary>
    /// Builds the Vertex AI endpoint URL.
    /// Uses the global endpoint when location is "global" for better availability.
    /// Google recommends global endpoint to reduce 429 errors by routing to
    /// the region with the most available capacity.
    /// </summary>
    private string BuildVertexUrl(string projectId, string location, string model, string method)
    {
        // Global endpoint: https://aiplatform.googleapis.com/v1/projects/{id}/locations/{loc}/...
        // Regional endpoint: https://{loc}-aiplatform.googleapis.com/v1/projects/{id}/locations/{loc}/...
        if (location == "global")
        {
            return $"https://aiplatform.googleapis.com/v1/projects/{projectId}/locations/global/publishers/google/models/{model}:{method}";
        }
        return $"https://{location}-aiplatform.googleapis.com/v1/projects/{projectId}/locations/{location}/publishers/google/models/{model}:{method}";
    }



    private IEnumerator SendVertexAI(string accessToken, string projectId, string location, string model, List<ChatMessage> messages, Action<string> onSuccess, Action<string> onError)
    {
        if (string.IsNullOrEmpty(model) || model.StartsWith("gpt")) model = "gemini-2.0-flash";
        string body = BuildGeminiBody(messages, IsWebSearchEnabled);

        List<string> targetRegions = GetTargetRegions(location);
        List<string> errors = new List<string>();

        // Iterate through regions immediately on failure
        for (int i = 0; i < targetRegions.Count; i++)
        {
            string currentRegion = targetRegions[i];
            string url = BuildVertexUrl(projectId, currentRegion, model, "generateContent");

            using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
            {
                byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
                req.uploadHandler = new UploadHandlerRaw(bodyRaw);
                req.downloadHandler = new DownloadHandlerBuffer();
                req.SetRequestHeader("Content-Type", "application/json");
                req.SetRequestHeader("Authorization", $"Bearer {accessToken}");
                req.timeout = 60;

                yield return req.SendWebRequest();

                if (req.result == UnityWebRequest.Result.Success)
                {
                    string response = ExtractGeminiResponse(req.downloadHandler.text);
                    onSuccess?.Invoke(response);
                    yield break; // Success — exit
                }

                // If 429 or 5xx, try next region immediately
                if (req.responseCode == 429 || req.responseCode >= 500)
                {
                    Debug.LogWarning($"[Vertex AI] {currentRegion} Error {req.responseCode}. Failing over to next region...");
                    errors.Add($"{currentRegion}: {req.responseCode}");
                    
                    // Tiny jitter to avoid thundering herd if many clients fail simultaneously
                    yield return new WaitForSeconds(UnityEngine.Random.Range(0.1f, 0.4f)); 
                    continue; 
                }

                // If 400/401/403 (Client Error), do not retry other regions (likely invalid request/auth)
                string errMsg = $"Vertex AI Error ({currentRegion}): {req.error}\n{req.downloadHandler.text}";
                onError?.Invoke(errMsg);
                yield break;
            }
        }

        // All regions failed
        onError?.Invoke($"Vertex AI: All regions failed ({string.Join(", ", targetRegions)}). Errors: {string.Join("; ", errors)}");
    }

    private IEnumerator SendVertexAIStreaming(string accessToken, string projectId, string location, string model, List<ChatMessage> messages, Action<string> onToken, Action<string> onComplete, Action<string> onError)
    {
        if (string.IsNullOrEmpty(model) || model.StartsWith("gpt")) model = "gemini-2.0-flash";
        string body = BuildGeminiBody(messages, IsWebSearchEnabled);

        List<string> targetRegions = GetTargetRegions(location);
        List<string> errors = new List<string>();

        // Iterate through regions immediately on failure
        for (int i = 0; i < targetRegions.Count; i++)
        {
            string currentRegion = targetRegions[i];
            string url = BuildVertexUrl(projectId, currentRegion, model, "streamGenerateContent") + "?alt=sse";

            using (UnityWebRequest req = new UnityWebRequest(url, "POST"))
            {
                byte[] bodyRaw = Encoding.UTF8.GetBytes(body);
                req.uploadHandler = new UploadHandlerRaw(bodyRaw);
                req.downloadHandler = new DownloadHandlerBuffer();
                req.SetRequestHeader("Content-Type", "application/json");
                req.SetRequestHeader("Authorization", $"Bearer {accessToken}");
                req.timeout = 120;

                var op = req.SendWebRequest();
                int lastProcessedIndex = 0;
                bool gotTokens = false;

                while (!op.isDone)
                {
                    if (IsAppForeground() && req.downloadHandler != null)
                    {
                        string currentData = req.downloadHandler.text;
                        if (currentData.Length > lastProcessedIndex)
                        {
                            string newData = currentData.Substring(lastProcessedIndex);
                            lastProcessedIndex = currentData.Length;
                            ExtractVertexStreamTokens(newData, onToken);
                            gotTokens = true;
                        }
                    }
                    if (IsAppForeground()) yield return null;
                    else yield return new WaitForSecondsRealtime(0.05f);
                }

                if (req.result == UnityWebRequest.Result.Success)
                {
                    // Process any remaining data
                    if (req.downloadHandler != null)
                    {
                        string finalData = req.downloadHandler.text;
                        if (finalData.Length > lastProcessedIndex)
                        {
                            ExtractVertexStreamTokens(finalData.Substring(lastProcessedIndex), onToken);
                        }
                    }
                    onComplete?.Invoke("");
                    yield break; // Success — exit
                }

                // If request failed but we ALREADY streamed tokens, we can't failover cleanly (user saw partial response)
                // In that case, we just have to error out or handle it gracefully.
                if (gotTokens)
                {
                     onError?.Invoke($"Vertex AI Stream Interrupted ({currentRegion}): {req.error}");
                     yield break;
                }

                // If 429 or 5xx AND no tokens received yet, try next region immediately
                if ((req.responseCode == 429 || req.responseCode >= 500) && !gotTokens)
                {
                    Debug.LogWarning($"[Vertex AI Stream] {currentRegion} Error {req.responseCode}. Failing over to next region...");
                    errors.Add($"{currentRegion}: {req.responseCode}");
                    yield return new WaitForSeconds(UnityEngine.Random.Range(0.1f, 0.4f));
                    continue; 
                }

                // Non-recoverable error
                string errMsg = $"Vertex AI Stream Error ({currentRegion}): {req.error}\n{req.downloadHandler.text}";
                onError?.Invoke(errMsg);
                yield break;
            }
        }

        // All regions failed
        onError?.Invoke($"Vertex AI Stream: All regions failed ({string.Join(", ", targetRegions)}). Errors: {string.Join("; ", errors)}");
    }

    private void ExtractVertexStreamTokens(string jsonChunk, Action<string> onToken)
    {
        // With ?alt=sse, each line starts with "data: " followed by JSON
        // Parse SSE lines and extract "text" from Gemini-format JSON
        string[] lines = jsonChunk.Split('\n');
        foreach (string rawLine in lines)
        {
            string line = rawLine.Trim();
            if (!line.StartsWith("data: ")) continue;
            string payload = line.Substring(6).Trim();
            if (payload == "[DONE]") continue;

            // Extract text from Gemini streaming JSON: "candidates":[{"content":{"parts":[{"text":"..."}]}}]
            int textIdx = payload.IndexOf("\"text\":");
            if (textIdx < 0) continue;
            int start = payload.IndexOf("\"", textIdx + 7) + 1;
            int end = FindClosingQuote(payload, start);
            if (end > start)
            {
                string text = UnescapeJson(payload.Substring(start, end - start));
                if (!string.IsNullOrEmpty(text))
                {
                    onToken?.Invoke(text);
                }
            }
        }
    }
}
