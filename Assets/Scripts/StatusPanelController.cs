using System.Collections;
using UnityEngine;
using UnityEngine.UI;
using TMPro;
using System;
using UnityEngine.InputSystem;

public class StatusPanelController : MonoBehaviour
{
    [Header("UI References")]
    public CanvasGroup panelCanvasGroup;
    public Button closeButton;

    [Header("Dynamic Texts")]
    public TextMeshProUGUI clockText;
    public TextMeshProUGUI cpuText;
    public TextMeshProUGUI memoryText;
    public TextMeshProUGUI llmInfoText; // [NEW] Displays Provider & Model
    public TextMeshProUGUI latencyText; // [NEW] Displays API Latency
    public TextMeshProUGUI networkText; // [NEW] Displays ONLINE or OFFLINE
    public TextMeshProUGUI nativeTtsText; // [NEW] Displays Native TTS status
    public TextMeshProUGUI nativeSttText; // [NEW] Displays Native STT status
    public TextMeshProUGUI nativeRagText; // [NEW] Displays Native RAG status

    [Header("Dynamic Bars")]
    public Slider syncSlider;
    public Slider memorySlider;
    public Slider cpuSlider;

    [Header("Settings")]
    public float fadeDuration = 0.3f;

    private Coroutine currentFadeCoroutine;
    private Action onCloseCallback;

    public bool IsActive => gameObject.activeSelf;

    private System.Diagnostics.Process currentProcess;
    private TimeSpan lastTotalProcessorTime;
    private DateTime lastCpuTimeCheck;
    private float currentCpuUsage = 0f;

    void Awake()
    {
        if (panelCanvasGroup == null) panelCanvasGroup = GetComponent<CanvasGroup>();
        
        // Init state
        if (panelCanvasGroup != null)
        {
            panelCanvasGroup.alpha = 0f;
            panelCanvasGroup.interactable = false;
            panelCanvasGroup.blocksRaycasts = false;
        }
        gameObject.SetActive(false);

        if (closeButton == null)
        {
            Transform btn = FindDeepChild(transform, "Btn_Close");
            if (btn != null) closeButton = btn.GetComponent<Button>();
        }

        if (closeButton) closeButton.onClick.AddListener(OnCloseClicked);

        // Init Diagnostics
        currentProcess = System.Diagnostics.Process.GetCurrentProcess();
        lastTotalProcessorTime = currentProcess.TotalProcessorTime;
        lastCpuTimeCheck = DateTime.Now;
        StartCoroutine(UpdateCpuUsageRoutine());

        // Auto-link network text if missing
        if (networkText == null)
        {
            Transform rt = transform.Find("InfoGrid/LeftCol/Row_NETWORK/Text");
            if (rt == null) 
            {
                // Try to find the second Text object with the positive local x position
                Transform row = transform.Find("InfoGrid/LeftCol/Row_NETWORK");
                if (row != null)
                {
                    foreach (Transform child in row)
                    {
                        if (child.name == "Text" && child.localPosition.x > 0)
                        {
                            rt = child;
                            break;
                        }
                    }
                }
            }
        if (rt != null)
        {
            networkText = rt.GetComponent<TextMeshProUGUI>();
        }
    }

        // Auto-link native feature status texts if missing
        AutoLinkStatusText("Row_NATIVE_TTS", ref nativeTtsText);
        AutoLinkStatusText("Row_NATIVE_STT", ref nativeSttText);
        AutoLinkStatusText("Row_NATIVE_RAG", ref nativeRagText);
    }

    private void AutoLinkStatusText(string rowName, ref TextMeshProUGUI target)
    {
        if (target != null) return;
        Transform row = transform.Find($"InfoGrid/LeftCol/{rowName}");
        if (row == null) row = transform.Find($"InfoGrid/RightCol/{rowName}");
        if (row == null) row = transform.Find(rowName);
        if (row != null)
        {
            foreach (Transform child in row)
            {
                if (child.name == "Text" && child.localPosition.x > 0)
                {
                    target = child.GetComponent<TextMeshProUGUI>();
                    break;
                }
            }
            if (target == null)
                target = row.GetComponentInChildren<TextMeshProUGUI>();
        }
    }

    void Update()
    {
        if (!IsActive) return;

        // Update Clock
        if (clockText) clockText.text = DateTime.Now.ToString("yyyy/MM/dd HH:mm:ss");

        UpdateMetrics();

        // Handle Backspace to close
        if (Keyboard.current != null && Keyboard.current.backspaceKey.wasPressedThisFrame)
        {
            OnCloseClicked();
        }
    }

    private IEnumerator UpdateCpuUsageRoutine()
    {
        while (true)
        {
            yield return new WaitForSeconds(0.5f);
            
            if (currentProcess != null)
            {
                currentProcess.Refresh();
                TimeSpan currentTotalProcessorTime = currentProcess.TotalProcessorTime;
                DateTime currentTime = DateTime.Now;

                double cpuUsedMs = (currentTotalProcessorTime - lastTotalProcessorTime).TotalMilliseconds;
                double totalTimeMs = (currentTime - lastCpuTimeCheck).TotalMilliseconds;
                
                // Normalized by processor count logic is tricky for "Game Load" vs "System Load"
                // But generally users expect "How much of the CPU is this App using?" or "Total System CPU?"
                // Process.TotalProcessorTime is just this App.
                // Let's calculate App CPU Usage relative to 1 Core first, or Total System?
                // Usually % is (CpuUsed / TotalTime) / Cores.
                
                double cpuUsage = (cpuUsedMs / totalTimeMs) / Environment.ProcessorCount;
                currentCpuUsage = Mathf.Clamp01((float)cpuUsage);

                lastTotalProcessorTime = currentTotalProcessorTime;
                lastCpuTimeCheck = currentTime;
            }
        }
    }

    private void UpdateMetrics()
    {
        // CPU (Updated via Coroutine)
        // Add a little noise to prevent static look if 0%
        float displayCpu = currentCpuUsage + UnityEngine.Random.Range(0f, 0.01f); 
        if (cpuSlider) cpuSlider.value = Mathf.Lerp(cpuSlider.value, displayCpu * 10f, Time.deltaTime * 5f); // Scale up for visibility? 
        // Actually, let's just show raw percentage. If it's low, it's low.
        if (cpuSlider) cpuSlider.value = Mathf.Lerp(cpuSlider.value, displayCpu, Time.deltaTime * 5f);
        if (cpuText) cpuText.text = $"CPU: {displayCpu * 100:F1} %";

        // Memory (Real Loop)
        long allocatedMemory = UnityEngine.Profiling.Profiler.GetTotalAllocatedMemoryLong();
        long reservedMemory = UnityEngine.Profiling.Profiler.GetTotalReservedMemoryLong();
        // long totalSystemMemory = SystemInfo.systemMemorySize * 1024L * 1024L; // in Bytes

        // Let's show Allocated vs Reserved or Allocated vs Random Max?
        // Usually "System Memory" implies total PC memory.
        // But "Memory" in a game status usually means "Game Memory Usage".
        // Let's show Allocated MB.
        
        double allocatedMB = allocatedMemory / (1024.0 * 1024.0);
        double reservedMB = reservedMemory / (1024.0 * 1024.0);
        
        // Use Reserved as the "Max" for the slider visualization? Or static 16GB?
        // Let's use 4GB (4096MB) as a visual baseline for the slider
        float memRatio = (float)(allocatedMB / 4096.0); 
        
        if (memorySlider) memorySlider.value = Mathf.Lerp(memorySlider.value, memRatio, Time.deltaTime * 2f);
        if (memoryText) memoryText.text = $"MEM: {allocatedMB:F0} MB";

        // Synchronization (FPS Stability)
        // Target 60fps
        float currentFPS = 1.0f / Time.smoothDeltaTime;
        float targetFPS = 60f; // Could check Screen.currentResolution.refreshRateRatio
        
        // Calculate stability: 1.0 is perfect 60.
        // If 30fps, sync is 50%? Or is sync just "Stability"?
        // Let's do (Current / Target)
        float syncVal = Mathf.Clamp01(currentFPS / targetFPS);
        
        // Add "Amadeus" flavor: If FPS is high, Sync is high.
        if (syncSlider) syncSlider.value = Mathf.Lerp(syncSlider.value, syncVal, Time.deltaTime * 3f);
        // Note: We removed the text for Sync in the builder, so no text update needed.

        // Network Status
        if (networkText)
        {
            if (Application.internetReachability == NetworkReachability.NotReachable)
            {
                networkText.text = "<color=#FF4444>OFFLINE</color>";
            }
            else
            {
                networkText.text = "<color=#44FF44>ONLINE</color>";
            }
        }

        // Native Feature Status
        UpdateNativeFeatureStatus();
    }

    private void UpdateNativeFeatureStatus()
    {
        int lang = PlayerPrefs.GetInt("Config_Language", 0);
        string on = lang == 0 ? "ON" : "ON";
        string off = lang == 0 ? "OFF" : "OFF";
        string gpu = lang == 0 ? "GPU" : "GPU";
        string cpu = lang == 0 ? "CPU" : "CPU";

        // Native TTS
        if (nativeTtsText)
        {
            bool enabled = NativeTTSService.Instance != null && NativeTTSService.Instance.IsEnabled;
            bool gpuCapable = enabled && NativeTTSService.Instance.UseGPU && NativeTTSService.Instance.IsGPUAvailable();
            string status = enabled ? $"<color=#44FF44>{on}</color>" : $"<color=#FF4444>{off}</color>";
            string gpuStatus = gpuCapable ? $" / <color=#44AAFF>{gpu}</color>" : (enabled ? $" / <color=#FFAA44>{cpu}</color>" : "");
            nativeTtsText.text = $"TTS: {status}{gpuStatus}";
        }

        // Native STT
        if (nativeSttText)
        {
            bool enabled = NativeSTTService.Instance != null && NativeSTTService.Instance.IsEnabled;
            bool gpuCapable = enabled && NativeSTTService.Instance.UseGPU && NativeSTTService.Instance.IsWhisperAvailable();
            string status = enabled ? $"<color=#44FF44>{on}</color>" : $"<color=#FF4444>{off}</color>";
            string gpuStatus = gpuCapable ? $" / <color=#44AAFF>{gpu}</color>" : (enabled ? $" / <color=#FFAA44>{cpu}</color>" : "");
            string recording = enabled && NativeSTTService.Instance.IsRecording ? " <color=#FF4444>[REC]</color>" : "";
            nativeSttText.text = $"STT: {status}{gpuStatus}{recording}";
        }

        // Native RAG
        if (nativeRagText)
        {
            bool enabled = NativeRAGService.Instance != null && NativeRAGService.Instance.IsEnabled;
            string status = enabled ? $"<color=#44FF44>{on}</color>" : $"<color=#FF4444>{off}</color>";
            nativeRagText.text = $"RAG: {status}";
        }
    }

    public void UpdateLLMStats(string provider, string model, float latencyMs)
    {
        if (llmInfoText)
        {
            // Format: "OpenAI / gpt-4o"
            llmInfoText.text = $"{provider} / {model}";
        }

        if (latencyText)
        {
            // Color code latency: Green < 1000ms, Yellow < 3000ms, Red > 3000ms
            string color = "green";
            if (latencyMs > 3000) color = "red";
            else if (latencyMs > 1000) color = "yellow";
            
            latencyText.text = $"Ping: <color={color}>{latencyMs:F0} ms</color>";
        }
    }

    public void Show(Action onClose = null)
    {
        onCloseCallback = onClose;
        gameObject.SetActive(true);
        UpdateLanguage();
        // Force immediate status refresh when panel opens
        UpdateNativeFeatureStatus();
        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvas(0f, 1f));
    }

    public void Hide()
    {
        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvas(1f, 0f, true));
        onCloseCallback?.Invoke();
    }

    private void OnCloseClicked()
    {
        Hide();
    }

    private IEnumerator FadeCanvas(float start, float end, bool disableOnFinish = false)
    {
        float t = 0f;
        if (panelCanvasGroup)
        {
            panelCanvasGroup.alpha = start;
            panelCanvasGroup.blocksRaycasts = (end > 0.5f);
            panelCanvasGroup.interactable = (end > 0.5f);
        }

        while (t < fadeDuration)
        {
            t += Time.deltaTime;
            if (panelCanvasGroup) panelCanvasGroup.alpha = Mathf.Lerp(start, end, t / fadeDuration);
            yield return null;
        }

        if (panelCanvasGroup) panelCanvasGroup.alpha = end;
        if (disableOnFinish) gameObject.SetActive(false);
    }

    /// <summary>
    /// Refreshes the text content based on the current language setting.
    /// </summary>
    public void UpdateLanguage()
    {
        int langIdx = PlayerPrefs.GetInt("Config_Language", 0);
        string getTr(string ja, string en, string zh, string ko, string es, string fr, string de, string ru)
        {
            if (langIdx == 1) return en;
            if (langIdx == 2) return zh;
            if (langIdx == 3) return ko;
            if (langIdx == 4) return es;
            if (langIdx == 5) return fr;
            if (langIdx == 6) return de;
            if (langIdx == 7) return ru;
            return ja;
        }

        string Close_JA = "閉じる";
        string Close_EN = "Close";
        string Close_ZH = "关闭";
        string Close_KO = "닫기";
        string Close_ES = "Cerrar";
        string Close_FR = "Fermer";
        string Close_DE = "Schließen";
        string Close_RU = "Закрыть";

        var closeButtonText = ResolveCloseButtonText();
        if (closeButtonText != null)
        {
            closeButtonText.text = GetSpacingTaggedText(getTr(Close_JA, Close_EN, Close_ZH, Close_KO, Close_ES, Close_FR, Close_DE, Close_RU), langIdx);
        }
    }

    private TextMeshProUGUI ResolveCloseButtonText()
    {
        if (closeButton == null)
        {
            Transform btn = FindDeepChild(transform, "Btn_Close");
            if (btn != null) closeButton = btn.GetComponent<Button>();
        }

        if (closeButton == null) return null;
        return closeButton.GetComponentInChildren<TextMeshProUGUI>(true);
    }

    private Transform FindDeepChild(Transform parent, string targetName)
    {
        if (parent == null) return null;
        if (parent.name == targetName) return parent;

        for (int i = 0; i < parent.childCount; i++)
        {
            Transform found = FindDeepChild(parent.GetChild(i), targetName);
            if (found != null) return found;
        }

        return null;
    }

    private string GetSpacingTaggedText(string text, int lang)
    {
        if (lang != 7 || string.IsNullOrEmpty(text)) return text;
        return System.Text.RegularExpressions.Regex.Replace(text, @"([\u0400-\u04FF]+)", "<cspace=-8.4px>$1</cspace>");
    }
}
