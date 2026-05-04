using System;
using System.Runtime.InteropServices;
using System.Threading;
using UnityEngine;

/// <summary>
/// Native Text-to-Speech service using Windows SAPI5 (ISpVoice).
/// Completely standalone - no Python server required.
/// Supports GPU flag for future GPU-accelerated backends.
/// </summary>
public class NativeTTSService : MonoBehaviour
{
    public static NativeTTSService Instance { get; private set; }

    [Header("Settings")]
    [SerializeField] private int speechRate = 0; // -10 to 10
    [SerializeField] private int volume = 100;   // 0 to 100

    // PlayerPrefs keys
    private const string PREF_NATIVE_TTS_ENABLED = "Config_NativeTTS_Enabled";
    private const string PREF_NATIVE_TTS_RATE = "Config_NativeTTS_Rate";
    private const string PREF_NATIVE_TTS_VOLUME = "Config_NativeTTS_Volume";
    private const string PREF_NATIVE_TTS_USE_GPU = "Config_NativeTTS_UseGPU";

    public bool IsEnabled
    {
        get => PlayerPrefs.GetInt(PREF_NATIVE_TTS_ENABLED, 0) == 1;
        set => PlayerPrefs.SetInt(PREF_NATIVE_TTS_ENABLED, value ? 1 : 0);
    }

    public int SpeechRate
    {
        get => PlayerPrefs.GetInt(PREF_NATIVE_TTS_RATE, speechRate);
        set => PlayerPrefs.SetInt(PREF_NATIVE_TTS_RATE, value);
    }

    public int SpeechVolume
    {
        get => PlayerPrefs.GetInt(PREF_NATIVE_TTS_VOLUME, volume);
        set => PlayerPrefs.SetInt(PREF_NATIVE_TTS_VOLUME, value);
    }

    public bool UseGPU
    {
        get => PlayerPrefs.GetInt(PREF_NATIVE_TTS_USE_GPU, 0) == 1;
        set => PlayerPrefs.SetInt(PREF_NATIVE_TTS_USE_GPU, value ? 1 : 0);
    }

    /// <summary>
    /// Check if GPU-accelerated TTS is available.
    /// Currently SAPI5 does not use GPU; this is a forward-compatible hook for ONNX-based TTS.
    /// </summary>
    public bool IsGPUAvailable()
    {
        // SAPI5 itself does not use GPU.
        // Future: check for ONNX Runtime + DirectML/CUDA
        return false;
    }

    public bool IsSpeaking { get; private set; }

    // --- SAPI5 COM Interop ---
    [ComImport, Guid("6C44DF74-72B9-4992-A1EC-EF996E0422D4"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface ISpVoice
    {
        int SetOutput(object pUnkOutput, bool fAllowFormatChanges);
        int GetOutputObjectToken(out IntPtr ppObjectToken);
        int GetOutputStream(out IntPtr ppStream);
        int Pause();
        int Resume();
        int SetVoice(IntPtr pToken);
        int GetVoice(out IntPtr ppToken);
        int Speak([MarshalAs(UnmanagedType.LPWStr)] string pwcs, uint dwFlags, out IntPtr pulStreamNumber);
        int SpeakStream(IntPtr pStream, uint dwFlags, out IntPtr pulStreamNumber);
        int GetStatus(out IntPtr pStatus, out IntPtr ppszLastBookmark);
        int Skip([MarshalAs(UnmanagedType.LPWStr)] string pItemType, int lNumItems, out IntPtr pulNumSkipped);
        int SetPriority(int ePriority);
        int GetPriority(out int pePriority);
        int SetAlertBoundary(int eBoundary);
        int GetAlertBoundary(out int peBoundary);
        int SetRate(int RateAdjust);
        int GetRate(out int pRateAdjust);
        int SetVolume(ushort usVolume);
        int GetVolume(out ushort pusVolume);
        int WaitUntilDone(uint msTimeout);
        int SetSyncSpeakTimeout(uint msTimeout);
        int GetSyncSpeakTimeout(out uint pmsTimeout);
        int SpeakCompleteEvent();
        int IsUISupported([MarshalAs(UnmanagedType.LPWStr)] string pszTypeOfUI, IntPtr pvExtraData, uint cbExtraData, out bool pfSupported);
        int DisplayUI(IntPtr hwndParent, [MarshalAs(UnmanagedType.LPWStr)] string pszTitle, [MarshalAs(UnmanagedType.LPWStr)] string pszTypeOfUI, IntPtr pvExtraData, uint cbExtraData);
    }

    // IL2CPP-safe COM class wrapper (avoids Type.GetTypeFromCLSID AOT issues)
    [ComImport, Guid("96749377-3391-11D2-9EE3-00C04F797396"), ClassInterface(ClassInterfaceType.None)]
    private class SpVoice { }

    private ISpVoice voice;
    private Thread speakThread;

    void Awake()
    {
        if (Instance != null && Instance != this)
        {
            Destroy(gameObject);
            return;
        }
        Instance = this;
    }

    void OnDestroy()
    {
        Stop();
        if (voice != null)
        {
            try { Marshal.ReleaseComObject(voice); } catch { }
            voice = null;
        }
    }

    /// <summary>
    /// Initialize the SAPI5 voice engine.
    /// </summary>
    public bool Initialize()
    {
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        try
        {
            if (voice == null)
            {
                // Use direct new for IL2CPP/AOT compatibility
                voice = (ISpVoice)new SpVoice();
                voice.SetRate(SpeechRate);
                voice.SetVolume((ushort)Mathf.Clamp(SpeechVolume, 0, 100));
            }
            return true;
        }
        catch (Exception ex)
        {
            Debug.LogError($"[NativeTTS] Failed to initialize SAPI5: {ex.Message}");
            return false;
        }
#else
        Debug.LogWarning("[NativeTTS] Native TTS is only supported on Windows.");
        return false;
#endif
    }

    /// <summary>
    /// Speak the given text asynchronously.
    /// </summary>
    public void Speak(string text, Action onComplete = null)
    {
        if (!IsEnabled) { onComplete?.Invoke(); return; }
        if (string.IsNullOrWhiteSpace(text)) { onComplete?.Invoke(); return; }

        if (voice == null && !Initialize())
        {
            onComplete?.Invoke();
            return;
        }

        Stop();

        IsSpeaking = true;
        speakThread = new Thread(() =>
        {
            try
            {
                IntPtr streamNum;
                uint flags = 0x0001 | 0x0008; // SPF_ASYNC | SPF_PURGEBEFORESPEAK
                voice.Speak(text, flags, out streamNum);
                voice.WaitUntilDone(uint.MaxValue);
            }
            catch (Exception ex)
            {
                Debug.LogError($"[NativeTTS] Speak error: {ex.Message}");
            }
            finally
            {
                IsSpeaking = false;
                if (onComplete != null)
                {
                    UnityMainThreadDispatcher.Enqueue(() => onComplete());
                }
            }
        });
        speakThread.IsBackground = true;
        speakThread.Start();
    }

    /// <summary>
    /// Stop current speech immediately.
    /// </summary>
    public void Stop()
    {
        if (voice != null)
        {
            try { voice.Speak(null, 0x0002, out _); } catch { } // SPF_PURGEBEFORESPEAK
        }
        if (speakThread != null && speakThread.IsAlive)
        {
            try { speakThread.Join(100); } catch { }
            speakThread = null;
        }
        IsSpeaking = false;
    }
}

/// <summary>
/// Simple main thread dispatcher for callbacks from background threads.
/// </summary>
public class UnityMainThreadDispatcher : MonoBehaviour
{
    private static readonly System.Collections.Generic.Queue<Action> actions = new System.Collections.Generic.Queue<Action>();
    private static UnityMainThreadDispatcher _instance;
    private static readonly object _lock = new object();

    public static void Enqueue(Action action)
    {
        lock (_lock)
        {
            actions.Enqueue(action);
        }
    }

    void Update()
    {
        lock (_lock)
        {
            while (actions.Count > 0)
            {
                try { actions.Dequeue()?.Invoke(); } catch (Exception e) { Debug.LogError(e); }
            }
        }
    }

    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]
    private static void Initialize()
    {
        if (_instance == null)
        {
            var go = new GameObject("UnityMainThreadDispatcher");
            _instance = go.AddComponent<UnityMainThreadDispatcher>();
            DontDestroyOnLoad(go);
        }
    }
}
