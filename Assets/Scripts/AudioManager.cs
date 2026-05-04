using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Networking;

/// <summary>
/// Manages audio playback for RealAmadeus including voice, BGM, and SE.
/// Provides lip-sync data extraction from audio playback.
/// </summary>
public class AudioManager : MonoBehaviour
{
    public static AudioManager Instance { get; private set; }

    [Header("Audio Sources")]
    [SerializeField] private AudioSource voiceAudioSource;
    [SerializeField] private AudioSource bgmAudioSource;
    [SerializeField] private AudioSource seAudioSource;

    [Header("Settings")]
    [SerializeField] private int lipSyncSampleSize = 256;
    [SerializeField] private float lipSyncSensitivity = 2.0f;

    // Volume properties (read from PlayerPrefs)
    public float MasterVolume
    {
        get => PlayerPrefs.GetFloat("Config_MasterVol", 1.0f);
        set
        {
            PlayerPrefs.SetFloat("Config_MasterVol", value);
            UpdateVolumes();
        }
    }

    public float BGMVolume
    {
        get => PlayerPrefs.GetFloat("Config_BGMVol", 0.8f);
        set
        {
            PlayerPrefs.SetFloat("Config_BGMVol", value);
            UpdateVolumes();
        }
    }

    public float SEVolume
    {
        get => PlayerPrefs.GetFloat("Config_SEVol", 1.0f);
        set
        {
            PlayerPrefs.SetFloat("Config_SEVol", value);
            UpdateVolumes();
        }
    }

    public float VoiceVolume
    {
        get => PlayerPrefs.GetFloat("Config_VoiceVol", 1.0f);
        set
        {
            PlayerPrefs.SetFloat("Config_VoiceVol", value);
            UpdateVolumes();
        }
    }

    // Lip sync value (0-1) for Live2D mouth animation
    public float CurrentLipSyncValue { get; private set; }

    // Events
    public event Action<float> OnLipSyncUpdate;
    public event Action OnVoicePlaybackComplete;

    private float[] lipSyncSamples;
    private bool isAnalyzingLipSync = false;
    private bool lowSpecMode = false;
    private int analysisFrameSkip = 0;

    void Awake()
    {
        if (Instance != null && Instance != this)
        {
            Destroy(gameObject);
            return;
        }
        Instance = this;

        // Create audio sources if not assigned
        if (voiceAudioSource == null)
        {
            var voiceObj = new GameObject("VoiceAudioSource");
            voiceObj.transform.SetParent(transform);
            voiceAudioSource = voiceObj.AddComponent<AudioSource>();
            voiceAudioSource.playOnAwake = false;
        }

        if (bgmAudioSource == null)
        {
            var bgmObj = new GameObject("BGMAudioSource");
            bgmObj.transform.SetParent(transform);
            bgmAudioSource = bgmObj.AddComponent<AudioSource>();
            bgmAudioSource.playOnAwake = false;
            bgmAudioSource.loop = true;
        }

        if (seAudioSource == null)
        {
            var seObj = new GameObject("SEAudioSource");
            seObj.transform.SetParent(transform);
            seAudioSource = seObj.AddComponent<AudioSource>();
            seAudioSource.playOnAwake = false;
        }

        lipSyncSamples = new float[lipSyncSampleSize];
        ApplyPerformanceMode(PlayerPrefs.GetInt("Config_LowSpecMode", 0) == 1);
        UpdateVolumes();
    }

    void Update()
    {
        if (isAnalyzingLipSync && voiceAudioSource != null && voiceAudioSource.isPlaying)
        {
            if (lowSpecMode && (++analysisFrameSkip % 2 != 0))
            {
                return;
            }
            AnalyzeLipSync();
        }
        else if (isAnalyzingLipSync && voiceAudioSource != null && !voiceAudioSource.isPlaying)
        {
            // Playback finished
            isAnalyzingLipSync = false;
            CurrentLipSyncValue = 0f;
            OnLipSyncUpdate?.Invoke(0f);
            OnVoicePlaybackComplete?.Invoke();
        }
    }

    /// <summary>
    /// Update all audio source volumes based on settings
    /// </summary>
    public void UpdateVolumes()
    {
        float master = MasterVolume;
        
        if (voiceAudioSource != null)
            voiceAudioSource.volume = VoiceVolume * master;
        
        if (bgmAudioSource != null)
            bgmAudioSource.volume = BGMVolume * master;
        
        if (seAudioSource != null)
            seAudioSource.volume = SEVolume * master;
    }

    public void ApplyPerformanceMode(bool lowSpec)
    {
        lowSpecMode = lowSpec;
        int targetSampleSize = lowSpecMode ? 128 : 256;
        if (lipSyncSampleSize != targetSampleSize)
        {
            lipSyncSampleSize = targetSampleSize;
            lipSyncSamples = new float[lipSyncSampleSize];
        }
    }

    /// <summary>
    /// Play voice audio from AudioClip
    /// </summary>
    public void PlayVoice(AudioClip clip, bool enableLipSync = true)
    {
        if (voiceAudioSource == null || clip == null) return;

        StopVoice();
        voiceAudioSource.clip = clip;
        voiceAudioSource.Play();
        
        if (enableLipSync)
        {
            isAnalyzingLipSync = true;
        }
    }

    /// <summary>
    /// Play voice audio from file path
    /// </summary>
    public void PlayVoiceFromFile(string filePath, bool enableLipSync = true, Action onComplete = null)
    {
        StartCoroutine(LoadAndPlayAudio(filePath, enableLipSync, onComplete));
    }

    /// <summary>
    /// Stop current voice playback
    /// </summary>
    public void StopVoice()
    {
        if (voiceAudioSource != null)
        {
            voiceAudioSource.Stop();
            voiceAudioSource.clip = null;
        }
        isAnalyzingLipSync = false;
        CurrentLipSyncValue = 0f;
        OnLipSyncUpdate?.Invoke(0f);
    }

    /// <summary>
    /// Check if voice is currently playing
    /// </summary>
    public bool IsVoicePlaying => voiceAudioSource != null && voiceAudioSource.isPlaying;

    /// <summary>
    /// Play BGM
    /// </summary>
    public void PlayBGM(AudioClip clip, bool loop = true)
    {
        if (bgmAudioSource == null || clip == null) return;

        bgmAudioSource.clip = clip;
        bgmAudioSource.loop = loop;
        bgmAudioSource.Play();
    }

    /// <summary>
    /// Stop BGM
    /// </summary>
    public void StopBGM()
    {
        if (bgmAudioSource != null)
        {
            bgmAudioSource.Stop();
        }
    }

    /// <summary>
    /// Play sound effect
    /// </summary>
    public void PlaySE(AudioClip clip)
    {
        if (seAudioSource == null || clip == null) return;
        seAudioSource.PlayOneShot(clip);
    }

    /// <summary>
    /// Load audio from file and play
    /// </summary>
    private IEnumerator LoadAndPlayAudio(string filePath, bool enableLipSync, Action onComplete)
    {
        if (string.IsNullOrEmpty(filePath))
        {
            Debug.LogError("[AudioManager] Empty file path");
            yield break;
        }

        // Convert to file URI
        string uri = filePath;
        if (!uri.StartsWith("file://") && !uri.StartsWith("http"))
        {
            uri = "file://" + filePath;
        }

        using (UnityWebRequest www = UnityWebRequestMultimedia.GetAudioClip(uri, AudioType.WAV))
        {
            yield return www.SendWebRequest();

            if (www.result != UnityWebRequest.Result.Success)
            {
                Debug.LogError($"[AudioManager] Failed to load audio: {www.error}");
                yield break;
            }

            AudioClip clip = DownloadHandlerAudioClip.GetContent(www);
            if (clip != null)
            {
                PlayVoice(clip, enableLipSync);
                
                if (onComplete != null)
                {
                    // Wait for playback to complete
                    while (voiceAudioSource.isPlaying)
                    {
                        yield return null;
                    }
                    onComplete?.Invoke();
                }
            }
        }
    }

    /// <summary>
    /// Analyze current audio output for lip sync
    /// </summary>
    private void AnalyzeLipSync()
    {
        if (voiceAudioSource == null || !voiceAudioSource.isPlaying) return;

        // Get spectrum data
        FFTWindow fftWindow = lowSpecMode ? FFTWindow.Rectangular : FFTWindow.BlackmanHarris;
        voiceAudioSource.GetSpectrumData(lipSyncSamples, 0, fftWindow);

        // Calculate average amplitude in voice frequency range
        float sum = 0f;
        int voiceFreqStart = lipSyncSampleSize / 16; // ~300Hz
        int voiceFreqEnd = lipSyncSampleSize / 4;    // ~2000Hz

        for (int i = voiceFreqStart; i < voiceFreqEnd; i++)
        {
            sum += lipSyncSamples[i];
        }

        float average = sum / (voiceFreqEnd - voiceFreqStart);
        
        // Apply sensitivity and clamp
        float previousValue = CurrentLipSyncValue;
        float targetValue = Mathf.Clamp01(average * lipSyncSensitivity * 100f);
        CurrentLipSyncValue = Mathf.Lerp(previousValue, targetValue, lowSpecMode ? 0.35f : 0.5f);

        OnLipSyncUpdate?.Invoke(CurrentLipSyncValue);
    }

    /// <summary>
    /// Get a procedural lip sync value for text-based animation (no audio)
    /// </summary>
    public static float GetProceduralLipSync(float time)
    {
        float mouth = Mathf.Abs(Mathf.Sin(time * 12f)) * 0.5f
                    + Mathf.Abs(Mathf.Sin(time * 7.3f)) * 0.3f
                    + Mathf.PerlinNoise(time * 8f, 5f) * 0.2f;
        return Mathf.Clamp01(mouth);
    }
}
