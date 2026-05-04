using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using UnityEngine;

#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
using UnityEngine.Windows.Speech;
#endif

/// <summary>
/// Native Speech-to-Text service.
/// Primary: Windows 10 built-in DictationRecognizer (native, no Python).
/// Optional: Whisper.cpp DLL with CUDA/Metal/DirectML for GPU acceleration.
/// </summary>
public class NativeSTTService : MonoBehaviour
{
    public static NativeSTTService Instance { get; private set; }

    [Header("Settings")]
    [SerializeField] private float silenceTimeout = 2.0f;
    [SerializeField] private int maxRecordingSeconds = 30;
    [SerializeField] private float silenceThreshold = 0.01f;
    [SerializeField] private string whisperModelPath = "whisper-base.bin";

    // PlayerPrefs keys
    private const string PREF_NATIVE_STT_ENABLED = "Config_NativeSTT_Enabled";
    private const string PREF_NATIVE_STT_USE_GPU = "Config_NativeSTT_UseGPU";
    private const string PREF_NATIVE_STT_LANGUAGE = "Config_NativeSTT_Language";

    public bool IsEnabled
    {
        get => PlayerPrefs.GetInt(PREF_NATIVE_STT_ENABLED, 0) == 1;
        set => PlayerPrefs.SetInt(PREF_NATIVE_STT_ENABLED, value ? 1 : 0);
    }

    public bool UseGPU
    {
        get => PlayerPrefs.GetInt(PREF_NATIVE_STT_USE_GPU, 0) == 1;
        set => PlayerPrefs.SetInt(PREF_NATIVE_STT_USE_GPU, value ? 1 : 0);
    }

    public string Language
    {
        get => PlayerPrefs.GetString(PREF_NATIVE_STT_LANGUAGE, "ja");
        set => PlayerPrefs.SetString(PREF_NATIVE_STT_LANGUAGE, value);
    }

    public bool IsRecording { get; private set; }
    public bool IsTranscribing { get; private set; }
    public float RecordingLevel { get; private set; }

    public event Action OnRecordingStarted;
    public event Action OnRecordingStopped;
    public event Action<string> OnTranscriptionComplete;
    public event Action<string> OnError;
    public event Action<float> OnRecordingLevelChanged;

#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
    private DictationRecognizer dictationRecognizer;
#endif

    private AudioClip recordingClip;
    private string microphoneName;
    private float lastSoundTime;
    private bool autoStop = true;
    private bool useWhisperMode = false;

    // ─── Whisper.cpp DLL imports ───
    private const string WHISPER_DLL = "whisper";
    private static bool whisperDllAvailable = false;

    [DllImport(WHISPER_DLL, CallingConvention = CallingConvention.Cdecl, EntryPoint = "whisper_init_from_file")]
    private static extern IntPtr WhisperInit(string path);

    [DllImport(WHISPER_DLL, CallingConvention = CallingConvention.Cdecl, EntryPoint = "whisper_free")]
    private static extern void WhisperFree(IntPtr ctx);

    [DllImport(WHISPER_DLL, CallingConvention = CallingConvention.Cdecl, EntryPoint = "whisper_full_default_params")]
    private static extern IntPtr WhisperFullDefaultParams(int strategy);

    [DllImport(WHISPER_DLL, CallingConvention = CallingConvention.Cdecl, EntryPoint = "whisper_full")]
    private static extern int WhisperFull(IntPtr ctx, IntPtr parameters, float[] samples, int n_samples);

    [DllImport(WHISPER_DLL, CallingConvention = CallingConvention.Cdecl, EntryPoint = "whisper_full_n_segments")]
    private static extern int WhisperFullNSegments(IntPtr ctx);

    [DllImport(WHISPER_DLL, CallingConvention = CallingConvention.Cdecl, EntryPoint = "whisper_full_get_segment_text")]
    private static extern IntPtr WhisperFullGetSegmentText(IntPtr ctx, int i_segment);

    /// <summary>
    /// Check if Whisper.cpp DLL is available on this system.
    /// </summary>
    public bool IsWhisperAvailable()
    {
        if (whisperDllAvailable) return true;
        try
        {
            // Try a no-op call to see if the DLL is present.
            // whisper_init_from_file with null will fail safely if the DLL is loaded,
            // but if the DLL is missing we get DllNotFoundException before entering.
            // Instead, we try to resolve the model path and call init.
            string modelPath = ResolveWhisperModelPath();
            if (string.IsNullOrEmpty(modelPath)) { whisperDllAvailable = false; return false; }
            IntPtr ctx = WhisperInit(modelPath);
            if (ctx != IntPtr.Zero)
            {
                WhisperFree(ctx);
                whisperDllAvailable = true;
            }
            else
            {
                whisperDllAvailable = false;
            }
        }
        catch (DllNotFoundException)
        {
            whisperDllAvailable = false;
        }
        catch { whisperDllAvailable = false; }
        return whisperDllAvailable;
    }

    void Awake()
    {
        if (Instance != null && Instance != this)
        {
            Destroy(gameObject);
            return;
        }
        Instance = this;
    }

    void Start()
    {
        if (Microphone.devices.Length > 0)
        {
            microphoneName = Microphone.devices[0];
        }
    }

    void Update()
    {
        if (IsRecording)
        {
            UpdateRecordingLevel();
            if (autoStop && Time.time - lastSoundTime > silenceTimeout)
            {
                StopRecording();
            }
        }
    }

    void OnDestroy()
    {
        StopRecording();
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        if (dictationRecognizer != null)
        {
            dictationRecognizer.Dispose();
            dictationRecognizer = null;
        }
#endif
    }

    /// <summary>
    /// Start voice recording and recognition.
    /// Prefers Whisper.cpp when GPU is requested and DLL is available.
    /// </summary>
    public void StartRecording(bool autoStopOnSilence = true)
    {
        if (!IsEnabled) { OnError?.Invoke("Native STT is not enabled"); return; }
        if (IsRecording) { Debug.LogWarning("[NativeSTT] Already recording"); return; }

        autoStop = autoStopOnSilence;
        lastSoundTime = Time.time;

        // Decide mode: Whisper (GPU capable) vs System Dictation
        useWhisperMode = UseGPU && IsWhisperAvailable();

        if (useWhisperMode)
        {
            StartWhisperRecording();
        }
        else
        {
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
            StartDictationRecording();
#else
            OnError?.Invoke("Native STT is only supported on Windows.");
#endif
        }
    }

    private string ResolveWhisperModelPath()
    {
        // 1. StreamingAssets
        string path = Path.Combine(Application.streamingAssetsPath, whisperModelPath);
        if (File.Exists(path)) return path;

        // 2. PersistentDataPath
        path = Path.Combine(Application.persistentDataPath, whisperModelPath);
        if (File.Exists(path)) return path;

        // 3. Relative to executable
        string exeDir = Directory.GetParent(Application.dataPath)?.FullName;
        if (!string.IsNullOrEmpty(exeDir))
        {
            path = Path.Combine(exeDir, whisperModelPath);
            if (File.Exists(path)) return path;
        }

        return null;
    }

    // ═══════════════════════════════════════════
    //  Whisper.cpp Recording
    // ═══════════════════════════════════════════

    private void StartWhisperRecording()
    {
        if (string.IsNullOrEmpty(microphoneName))
        {
            OnError?.Invoke("No microphone available");
            return;
        }

        recordedSamples.Clear();
        recordingClip = Microphone.Start(microphoneName, false, maxRecordingSeconds, 16000);
        if (recordingClip == null)
        {
            OnError?.Invoke("Failed to start microphone recording");
            return;
        }

        IsRecording = true;
        OnRecordingStarted?.Invoke();
    }

    private IEnumerator RunWhisperTranscription(float[] samples)
    {
        IsTranscribing = true;
        string result = null;
        string error = null;

        Thread thread = new Thread(() =>
        {
            IntPtr ctx = IntPtr.Zero;
            try
            {
                string modelPath = ResolveWhisperModelPath();
                ctx = WhisperInit(modelPath);
                if (ctx == IntPtr.Zero)
                {
                    error = "Failed to initialize Whisper context";
                    return;
                }

                IntPtr wparams = WhisperFullDefaultParams(0); // greedy
                // Note: wparams is an unmanaged struct pointer. In a production build,
                // you should marshal and set language, threads, translate flags here.
                // For now we use defaults.

                int ret = WhisperFull(ctx, wparams, samples, samples.Length);
                if (ret != 0)
                {
                    error = $"Whisper inference failed (code {ret})";
                    return;
                }

                int nSegments = WhisperFullNSegments(ctx);
                StringBuilder sb = new StringBuilder();
                for (int i = 0; i < nSegments; i++)
                {
                    IntPtr pText = WhisperFullGetSegmentText(ctx, i);
                    if (pText != IntPtr.Zero)
                    {
                        string segment = Marshal.PtrToStringAnsi(pText);
                        sb.Append(segment);
                    }
                }
                result = sb.ToString().Trim();
            }
            catch (Exception ex)
            {
                error = $"Whisper exception: {ex.Message}";
            }
            finally
            {
                if (ctx != IntPtr.Zero) WhisperFree(ctx);
            }
        });
        thread.IsBackground = true;
        thread.Start();

        while (thread.IsAlive)
            yield return null;

        IsTranscribing = false;

        if (!string.IsNullOrEmpty(error))
        {
            Debug.LogError($"[NativeSTT] {error}");
            OnError?.Invoke(error);
        }
        else if (!string.IsNullOrWhiteSpace(result))
        {
            OnTranscriptionComplete?.Invoke(result);
        }
    }

    private List<float> recordedSamples = new List<float>();

    // ═══════════════════════════════════════════
    //  DictationRecognizer Recording
    // ═══════════════════════════════════════════

#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
    private void StartDictationRecording()
    {
        if (dictationRecognizer != null)
        {
            dictationRecognizer.Dispose();
        }

        dictationRecognizer = new DictationRecognizer();
        dictationRecognizer.AutoSilenceTimeoutSeconds = silenceTimeout;
        dictationRecognizer.InitialSilenceTimeoutSeconds = maxRecordingSeconds;

        string resultText = "";
        bool gotResult = false;
        bool errorOccurred = false;
        string errorText = "";

        dictationRecognizer.DictationResult += (text, confidence) =>
        {
            resultText = text;
            gotResult = true;
        };

        dictationRecognizer.DictationError += (error, hresult) =>
        {
            errorText = error;
            errorOccurred = true;
        };

        dictationRecognizer.DictationComplete += (cause) =>
        {
            // Completion handled in coroutine
        };

        dictationRecognizer.Start();
        IsRecording = true;
        OnRecordingStarted?.Invoke();

        StartCoroutine(WaitForDictation(resultText, gotResult, errorOccurred, errorText));
    }

    private IEnumerator WaitForDictation(string resultText, bool gotResult, bool errorOccurred, string errorText)
    {
        float elapsed = 0f;
        while (IsRecording && elapsed < maxRecordingSeconds + 2f)
        {
            yield return null;
            elapsed += Time.deltaTime;
        }

        if (dictationRecognizer != null && dictationRecognizer.Status == SpeechSystemStatus.Running)
        {
            dictationRecognizer.Stop();
        }

        IsRecording = false;
        RecordingLevel = 0f;
        OnRecordingStopped?.Invoke();

        if (errorOccurred)
        {
            OnError?.Invoke(errorText);
        }
        else if (!string.IsNullOrWhiteSpace(resultText))
        {
            OnTranscriptionComplete?.Invoke(resultText.Trim());
        }

        if (dictationRecognizer != null)
        {
            dictationRecognizer.Dispose();
            dictationRecognizer = null;
        }
    }
#endif

    /// <summary>
    /// Stop recording manually.
    /// </summary>
    public void StopRecording()
    {
        if (!IsRecording) return;
        IsRecording = false;
        RecordingLevel = 0f;

        if (useWhisperMode)
        {
            // Stop microphone and process
            int position = Microphone.GetPosition(microphoneName);
            Microphone.End(microphoneName);

            if (position > 0 && recordingClip != null)
            {
                float[] samples = new float[position];
                recordingClip.GetData(samples, 0);
                OnRecordingStopped?.Invoke();
                StartCoroutine(RunWhisperTranscription(samples));
            }
            else
            {
                OnRecordingStopped?.Invoke();
                OnError?.Invoke("No audio recorded");
            }
            return;
        }

#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        if (dictationRecognizer != null && dictationRecognizer.Status == SpeechSystemStatus.Running)
        {
            dictationRecognizer.Stop();
        }
#endif

        if (Microphone.IsRecording(microphoneName))
        {
            Microphone.End(microphoneName);
        }

        OnRecordingStopped?.Invoke();
    }

    private void UpdateRecordingLevel()
    {
        if (useWhisperMode && recordingClip != null)
        {
            int position = Microphone.GetPosition(microphoneName);
            if (position <= 0) return;
            int sampleCount = Mathf.Min(256, position);
            float[] samples = new float[sampleCount];
            recordingClip.GetData(samples, position - sampleCount);
            float sum = 0f;
            for (int i = 0; i < samples.Length; i++) sum += samples[i] * samples[i];
            RecordingLevel = Mathf.Sqrt(sum / samples.Length);
            if (RecordingLevel > silenceThreshold) lastSoundTime = Time.time;
            OnRecordingLevelChanged?.Invoke(RecordingLevel);
        }
    }
}
