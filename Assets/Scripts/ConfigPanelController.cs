using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.EventSystems;
using TMPro;
using System;

public class ConfigPanelController : MonoBehaviour
{
    [Header("UI References")]
    public CanvasGroup panelCanvasGroup;
    public TextMeshProUGUI headerText;
    public List<GameObject> categoryPages; // 0:General, 1:Text, 2:Sound, 3:Graphic, 4:API

    [Header("General Settings")]
    public Toggle skipLoadingToggle;
    public Toggle rightClickMenuToggle;
    public Toggle desktopNotificationsToggle;
    public Toggle lightweightModeToggle;
    public Toggle gazeTrackingToggle;
    public Toggle desktopBackgroundModeToggle;
    public TMP_Dropdown languageDropdown;
    public TextMeshProUGUI languageLabelText;

    [Header("Text Settings")]
    public Slider textSpeedSlider;
    public TextMeshProUGUI textSpeedValueText;
    public Slider autoSpeedSlider;
    public TextMeshProUGUI autoSpeedValueText;
    public Toggle autoModeToggle; // Added Auto Mode Toggle

    [Header("Sound Settings")]
    public Slider masterVolumeSlider;
    public TextMeshProUGUI masterVolumeValueText;
    public Slider bgmVolumeSlider;
    public TextMeshProUGUI bgmVolumeValueText;
    public Slider seVolumeSlider;
    public TextMeshProUGUI seVolumeValueText;
    public Slider voiceVolumeSlider;
    public TextMeshProUGUI voiceVolumeValueText;

    [Header("Graphic Settings")]
    public TMP_Dropdown screenModeDropdown;
    public TMP_Dropdown resolutionDropdown;

    [Header("API Settings")]
    public TMP_Dropdown apiProviderDropdown;
    public TMP_InputField apiKeyInputField;
    public TMP_InputField modelNameInputField;
    public Toggle webSearchToggle;
    [Header("Vertex AI Settings")]
    public TMP_InputField vertexProjectInputField;
    public TMP_InputField vertexLocationInputField;
    public TMP_InputField vertexClientIdInputField; // For WebGL
    public TextMeshProUGUI vertexInfoText;
    public Toggle vertexUseGcloudToggle; // To switch between Web Auth and gcloud
    public Button vertexAuthButton;

    [Header("Ollama Settings")]
    public TMP_InputField ollamaHostInputField;

    [Header("Native Experimental Settings (No Python / No Server)")]    public Toggle toolsEnabledToggle;
    public Toggle nativeTtsEnabledToggle;
    public Toggle nativeTtsGpuToggle;
    public Toggle nativeSttEnabledToggle;
    public Toggle nativeSttGpuToggle;
    public TMP_InputField nativeSttLanguageInput;
    public Toggle nativeRagEnabledToggle;
    public TMP_InputField nativeRagTopKInput;
    public TMP_InputField nativeRagThresholdInput;

    [Header("Common")]
    public Button saveButton;
    public Button cancelButton;

    [Header("Sidebar")]
    public List<Button> categoryButtons;

    [Header("Settings")]
    public float fadeDuration = 0.3f;

    // Keys for PlayerPrefs (Updated)
    private const string PREF_SKIP_LOADING = "Config_SkipLoading";
    private const string PREF_RIGHT_CLICK_MENU = "Config_RightClickMenu";
    private const string PREF_DESKTOP_NOTIFICATIONS = "Config_Notifications";
    private const string PREF_LOW_SPEC_MODE = "Config_LowSpecMode";
    private const string PREF_GAZE_TRACKING = "Config_GazeTracking";
    private const string PREF_LANGUAGE = "Config_Language";
    private const string PREF_TEXT_SPEED = "Config_TextSpeed";
    private const string PREF_AUTO_SPEED = "Config_AutoSpeed";
    private const string PREF_AUTO_MODE = "Config_AutoMode"; // Added Pref Key
    private const string PREF_MASTER_VOL = "Config_MasterVol";
    private const string PREF_BGM_VOL = "Config_BGMVol";
    private const string PREF_SE_VOL = "Config_SEVol";
    private const string PREF_VOICE_VOL = "Config_VoiceVol";
    private const string PREF_SCREEN_MODE = "Config_ScreenMode";
    private const string PREF_RESOLUTION = "Config_Resolution";
    private const string PREF_API_PROVIDER = "Config_ApiProvider";
    private const string PREF_API_KEY = "Config_ApiKey";
    private const string PREF_MODEL_NAME = "Config_ModelName";
    private const string PREF_WEB_SEARCH = "Config_WebSearch";
    private const string PREF_VERTEX_PROJECT = "Config_VertexProject";
    private const string PREF_VERTEX_LOCATION = "Config_VertexLocation";
    private const string PREF_OLLAMA_HOST = "Config_OllamaHost";
    
    public const string PREF_TOOLS_ENABLED = "Config_Experimental_ToolCalling_Enabled";
    private const string PREF_DESKTOP_BACKGROUND_MODE = "Config_Experimental_DesktopBackgroundMode";
    private const string PREF_NATIVE_TTS_ENABLED     = "Config_NativeTTS_Enabled";
    private const string PREF_NATIVE_TTS_GPU         = "Config_NativeTTS_UseGPU";
    private const string PREF_NATIVE_STT_ENABLED     = "Config_NativeSTT_Enabled";
    private const string PREF_NATIVE_STT_GPU         = "Config_NativeSTT_UseGPU";
    private const string PREF_NATIVE_STT_LANGUAGE    = "Config_NativeSTT_Language";
    private const string PREF_NATIVE_RAG_ENABLED     = "Config_NativeRAG_Enabled";
    private const string PREF_NATIVE_RAG_TOPK        = "Config_NativeRAG_TopK";
    private const string PREF_NATIVE_RAG_THRESHOLD   = "Config_NativeRAG_Threshold";

    private Coroutine currentFadeCoroutine;
    private Action onCloseCallback;
    private int activeCategoryIndex = 0;
    
    // Multiple API Key and Model Name Support
    private Dictionary<int, string> apiKeys = new Dictionary<int, string>();
    private Dictionary<int, string> modelNames = new Dictionary<int, string>();
    private int currentApiProviderIndex = 0;
    private int currentLanguageIndex = 0; // 0:ja 1:en 2:zh 3:ko 4:es 5:fr 6:de 7:ru
    private const string PREF_API_KEY_PREFIX = "Config_ApiKey_";
    private const string PREF_MODEL_NAME_PREFIX = "Config_ModelName_";

    public bool IsActive => gameObject.activeSelf;

    void Awake()
    {
        if (panelCanvasGroup == null) panelCanvasGroup = GetComponent<CanvasGroup>();
        
        // Setup default state
        if (panelCanvasGroup != null)
        {
            panelCanvasGroup.alpha = 0f;
            panelCanvasGroup.interactable = false;
            panelCanvasGroup.blocksRaycasts = false;
        }
        gameObject.SetActive(false);

        // Add Listeners
        if (saveButton) saveButton.onClick.AddListener(OnSaveClicked);
        if (cancelButton) cancelButton.onClick.AddListener(OnCancelClicked);
        
        // Sliders
        if (textSpeedSlider) textSpeedSlider.onValueChanged.AddListener(val => UpdateSliderText(textSpeedValueText, val, "{0:P0}"));
        if (autoSpeedSlider) autoSpeedSlider.onValueChanged.AddListener(val => UpdateSliderText(autoSpeedValueText, val, "{0:F1}s"));
        if (masterVolumeSlider) masterVolumeSlider.onValueChanged.AddListener(val => UpdateSliderText(masterVolumeValueText, val, "{0:P0}"));
        if (bgmVolumeSlider) bgmVolumeSlider.onValueChanged.AddListener(val => UpdateSliderText(bgmVolumeValueText, val, "{0:P0}"));
        if (seVolumeSlider) seVolumeSlider.onValueChanged.AddListener(val => UpdateSliderText(seVolumeValueText, val, "{0:P0}"));
        if (voiceVolumeSlider) voiceVolumeSlider.onValueChanged.AddListener(val => UpdateSliderText(voiceVolumeValueText, val, "{0:P0}"));

        // Category Buttons
        if (categoryButtons != null)
        {
            for (int i = 0; i < categoryButtons.Count; i++)
            {
                int index = i; // Local copy for closure
                categoryButtons[i].onClick.AddListener(() => OnCategoryClicked(index));
            }
        }

        // Initialize Selection (Default to General)
        OnCategoryClicked(0);

        // Populate Dropdowns if empty
        if (apiProviderDropdown != null)
        {
            if (apiProviderDropdown.options.Count == 0)
            {
                apiProviderDropdown.AddOptions(new List<string> { "OpenAI", "Google Gemini", "Anthropic Claude", "Groq", "Vertex AI", "Ollama", "OpenRouter" });
            }

            apiProviderDropdown.onValueChanged.AddListener(OnApiProviderChanged);
        }
        if (screenModeDropdown != null && screenModeDropdown.options.Count == 0)
        {
            screenModeDropdown.AddOptions(new List<string> { "Windowed", "FullScreen", "Borderless" });
        }
        if (resolutionDropdown != null && resolutionDropdown.options.Count == 0)
        {
            resolutionDropdown.AddOptions(new List<string> { "1920x1080", "1280x720", "Text Window Only" });
        }

        SetupLanguageDropdown();

        ApplySettings(); // Apply settings on startup

        if (vertexAuthButton) vertexAuthButton.onClick.AddListener(OnVertexAuthClicked);
        if (vertexUseGcloudToggle) vertexUseGcloudToggle.onValueChanged.AddListener((val) => UpdateProviderFieldsVisibility(currentApiProviderIndex));
    }

    private void OnVertexAuthClicked()
    {
        // Save first so Client ID is updated
        SaveSettings();
        if (VertexOAuthService.Instance != null)
        {
            VertexOAuthService.Instance.Authenticate(
                () => Debug.Log("Vertex Auth Success"),
                (err) => 
                {
                    Debug.LogError("Vertex Auth Error: " + err);
                    FindObjectOfType<AmadeusChatController>()?.OnAPIError(err);
                }
            );
        }
    }

    void Update()
    {
        if (!IsActive) return;

        // Check if user is typing in an input field
        if (EventSystem.current != null && EventSystem.current.currentSelectedGameObject != null)
        {
            if (EventSystem.current.currentSelectedGameObject.GetComponent<TMP_InputField>() != null)
                return;
        }

        if (Input.GetKeyDown(KeyCode.W) || Input.GetKeyDown(KeyCode.UpArrow))
        {
            CycleCategory(-1);
        }
        else if (Input.GetKeyDown(KeyCode.S) || Input.GetKeyDown(KeyCode.DownArrow))
        {
            CycleCategory(1);
        }

        if (Input.GetKeyDown(KeyCode.Backspace))
        {
            OnSaveClicked();
        }
    }

    private void CycleCategory(int direction)
    {
        if (categoryButtons == null || categoryButtons.Count == 0) return;

        int newIndex = activeCategoryIndex + direction;
        
        // Handle wrapping
        if (newIndex < 0) newIndex = categoryButtons.Count - 1;
        else if (newIndex >= categoryButtons.Count) newIndex = 0;

        OnCategoryClicked(newIndex);
    }

    private void UpdateSliderText(TextMeshProUGUI textComp, float value, string format)
    {
        if (textComp) textComp.text = string.Format(format, value);
    }

    private void OnCategoryClicked(int index)
    {
        activeCategoryIndex = index;
        
        // Update Header
        if (headerText)
        {
            headerText.text = GetLocalizedCategoryHeader(index);
        }

        // Switch Pages
        if (categoryPages != null)
        {
            for (int i = 0; i < categoryPages.Count; i++)
            {
                if (categoryPages[i] != null)
                    categoryPages[i].SetActive(i == index);
            }
        }

        UpdateSidebarVisuals();
    }

    private void UpdateSidebarVisuals()
    {
        if (categoryButtons == null) return;

        for (int i = 0; i < categoryButtons.Count; i++)
        {
            var btn = categoryButtons[i];
            if (btn == null) continue;

            var outline = btn.GetComponent<Outline>();
            if (outline != null)
            {
                outline.enabled = (i == activeCategoryIndex);
            }
        }
    }

    public void Show(Action onClose = null)
    {
        onCloseCallback = onClose;
        LoadSettings(); // Load data into UI
        gameObject.SetActive(true);
        OnCategoryClicked(activeCategoryIndex); // Refresh view
        DisableAllOutlines(); // Hide outlines during fade to prevent ghosting
        
        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvas(0f, 1f, false, true));
    }

    public void Hide()
    {
        DisableAllOutlines(); // Hide outlines immediately to prevent "white ghosting" during fade
        if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
        currentFadeCoroutine = StartCoroutine(FadeCanvas(1f, 0f, true));
    }

    private void DisableAllOutlines()
    {
        if (categoryButtons == null) return;
        foreach (var btn in categoryButtons)
        {
            if (btn == null) continue;
            var outline = btn.GetComponent<Outline>();
            if (outline != null) outline.enabled = false;
        }
    }

    private void LoadSettings()
    {
        currentLanguageIndex = PlayerPrefs.GetInt(PREF_LANGUAGE, 0);

        // Add safe checks and defaults
        if (skipLoadingToggle) skipLoadingToggle.isOn = PlayerPrefs.GetInt(PREF_SKIP_LOADING, 0) == 1;
        if (rightClickMenuToggle) rightClickMenuToggle.isOn = PlayerPrefs.GetInt(PREF_RIGHT_CLICK_MENU, 1) == 1;
        if (desktopNotificationsToggle) desktopNotificationsToggle.isOn = PlayerPrefs.GetInt(PREF_DESKTOP_NOTIFICATIONS, 1) == 1;
        if (lightweightModeToggle) lightweightModeToggle.isOn = PlayerPrefs.GetInt(PREF_LOW_SPEC_MODE, 0) == 1;
        if (gazeTrackingToggle) gazeTrackingToggle.isOn = PlayerPrefs.GetInt(PREF_GAZE_TRACKING, 0) == 1;
        if (languageDropdown) languageDropdown.SetValueWithoutNotify(currentLanguageIndex);
        
        if (textSpeedSlider) { textSpeedSlider.value = PlayerPrefs.GetFloat(PREF_TEXT_SPEED, 1.0f); UpdateSliderText(textSpeedValueText, textSpeedSlider.value, "{0:P0}"); }
        if (textSpeedSlider) { textSpeedSlider.value = PlayerPrefs.GetFloat(PREF_TEXT_SPEED, 1.0f); UpdateSliderText(textSpeedValueText, textSpeedSlider.value, "{0:P0}"); }
        if (autoSpeedSlider) { autoSpeedSlider.value = PlayerPrefs.GetFloat(PREF_AUTO_SPEED, 3.0f); UpdateSliderText(autoSpeedValueText, autoSpeedSlider.value, "{0:F1}s"); }
        if (autoModeToggle) autoModeToggle.isOn = PlayerPrefs.GetInt(PREF_AUTO_MODE, 0) == 1; // Load Auto Mode

        if (masterVolumeSlider) { masterVolumeSlider.value = PlayerPrefs.GetFloat(PREF_MASTER_VOL, 1.0f); UpdateSliderText(masterVolumeValueText, masterVolumeSlider.value, "{0:P0}"); }
        if (bgmVolumeSlider) { bgmVolumeSlider.value = PlayerPrefs.GetFloat(PREF_BGM_VOL, 0.8f); UpdateSliderText(bgmVolumeValueText, bgmVolumeSlider.value, "{0:P0}"); }
        if (seVolumeSlider) { seVolumeSlider.value = PlayerPrefs.GetFloat(PREF_SE_VOL, 1.0f); UpdateSliderText(seVolumeValueText, seVolumeSlider.value, "{0:P0}"); }
        if (voiceVolumeSlider) { voiceVolumeSlider.value = PlayerPrefs.GetFloat(PREF_VOICE_VOL, 1.0f); UpdateSliderText(voiceVolumeValueText, voiceVolumeSlider.value, "{0:P0}"); }

        if (screenModeDropdown) screenModeDropdown.value = PlayerPrefs.GetInt(PREF_SCREEN_MODE, 0);
        if (resolutionDropdown) resolutionDropdown.value = PlayerPrefs.GetInt(PREF_RESOLUTION, 0);

        if (apiProviderDropdown) 
        {
            apiProviderDropdown.value = PlayerPrefs.GetInt(PREF_API_PROVIDER, 0);
            currentApiProviderIndex = apiProviderDropdown.value;

            // Load logic: Buffer all keys
            // The dropdown options count gives us the range (usually 5)
            int providerCount = apiProviderDropdown.options.Count;
            for (int i = 0; i < providerCount; i++)
            {
                string key = SecurePrefs.GetProtectedString(PREF_API_KEY_PREFIX + i, PlayerPrefs.GetString(PREF_API_KEY_PREFIX + i, ""));
                string modelName = PlayerPrefs.GetString(PREF_MODEL_NAME_PREFIX + i, "");
                
                // Legacy Migration: If Groq (3) is explicitly empty, check old global key
                // Or just try to migrate global key if specific key is empty
                if (string.IsNullOrEmpty(key) && i == currentApiProviderIndex)
                {
                     string legacyKey = SecurePrefs.GetProtectedString(PREF_API_KEY, PlayerPrefs.GetString(PREF_API_KEY, ""));
                     if (!string.IsNullOrEmpty(legacyKey)) key = legacyKey;
                }
                apiKeys[i] = key;

                if (string.IsNullOrEmpty(modelName) && i == currentApiProviderIndex)
                {
                     string legacyModel = PlayerPrefs.GetString(PREF_MODEL_NAME, "");
                     if (!string.IsNullOrEmpty(legacyModel)) modelName = legacyModel;
                }
                modelNames[i] = modelName;
            }

            // Set input field to current provider's buffer
            if (apiKeyInputField) apiKeyInputField.text = apiKeys.ContainsKey(currentApiProviderIndex) ? apiKeys[currentApiProviderIndex] : "";
            if (modelNameInputField) modelNameInputField.text = modelNames.ContainsKey(currentApiProviderIndex) ? modelNames[currentApiProviderIndex] : "";
        }
        
        if (webSearchToggle) webSearchToggle.isOn = PlayerPrefs.GetInt(PREF_WEB_SEARCH, 0) == 1;

        if (vertexProjectInputField) vertexProjectInputField.text = PlayerPrefs.GetString(PREF_VERTEX_PROJECT, "");
        if (vertexLocationInputField) vertexLocationInputField.text = PlayerPrefs.GetString(PREF_VERTEX_LOCATION, "us-central1");
        if (ollamaHostInputField) ollamaHostInputField.text = PlayerPrefs.GetString(PREF_OLLAMA_HOST, "http://localhost:11434");
        if (vertexClientIdInputField) vertexClientIdInputField.text = PlayerPrefs.GetString(VertexOAuthService.PREF_VERTEX_CLIENT_ID, "");
        if (vertexUseGcloudToggle) vertexUseGcloudToggle.isOn = PlayerPrefs.GetInt("Config_VertexUseGcloud", 0) == 1;
        if (desktopBackgroundModeToggle) desktopBackgroundModeToggle.isOn = PlayerPrefs.GetInt(PREF_DESKTOP_BACKGROUND_MODE, 0) == 1;

        UpdateProviderFieldsVisibility(currentApiProviderIndex);
        ApplyConfigLanguage(currentLanguageIndex);
        
        // Load experimental settings
        LoadExperimentalSettings();
    }
    
    private void LoadExperimentalSettings()
    {
        if (toolsEnabledToggle) toolsEnabledToggle.isOn = PlayerPrefs.GetInt(PREF_TOOLS_ENABLED, 0) == 1;

        if (nativeTtsEnabledToggle) nativeTtsEnabledToggle.isOn = PlayerPrefs.GetInt(PREF_NATIVE_TTS_ENABLED, 0) == 1;
        if (nativeTtsGpuToggle) nativeTtsGpuToggle.isOn = PlayerPrefs.GetInt(PREF_NATIVE_TTS_GPU, 0) == 1;
        if (nativeSttEnabledToggle) nativeSttEnabledToggle.isOn = PlayerPrefs.GetInt(PREF_NATIVE_STT_ENABLED, 0) == 1;
        if (nativeSttGpuToggle) nativeSttGpuToggle.isOn = PlayerPrefs.GetInt(PREF_NATIVE_STT_GPU, 0) == 1;
        if (nativeSttLanguageInput) nativeSttLanguageInput.text = PlayerPrefs.GetString(PREF_NATIVE_STT_LANGUAGE, "ja-JP");
        if (nativeRagEnabledToggle) nativeRagEnabledToggle.isOn = PlayerPrefs.GetInt(PREF_NATIVE_RAG_ENABLED, 0) == 1;
        if (nativeRagTopKInput) nativeRagTopKInput.text = PlayerPrefs.GetInt(PREF_NATIVE_RAG_TOPK, 5).ToString();
        if (nativeRagThresholdInput) nativeRagThresholdInput.text = PlayerPrefs.GetFloat(PREF_NATIVE_RAG_THRESHOLD, 0.3f).ToString("F2");
    }

    public void OnApiProviderChanged(int newIndex)
    {
        // Save current input to buffer for old index
        if (apiKeyInputField)
        {
            apiKeys[currentApiProviderIndex] = apiKeyInputField.text;
        }
        if (modelNameInputField)
        {
            modelNames[currentApiProviderIndex] = modelNameInputField.text;
        }

        // Update index
        currentApiProviderIndex = newIndex;

        // Load new buffer to input
        if (apiKeyInputField)
        {
            string key = apiKeys.ContainsKey(newIndex) ? apiKeys[newIndex] : "";
            apiKeyInputField.text = key;
        }
        if (modelNameInputField)
        {
            string modelName = modelNames.ContainsKey(newIndex) ? modelNames[newIndex] : "";
            modelNameInputField.text = modelName;
        }

        UpdateProviderFieldsVisibility(newIndex);
    }

    /// <summary>
    /// Shows/hides provider-specific fields based on selected provider.
    /// </summary>
    private void UpdateProviderFieldsVisibility(int providerIndex)
    {
        bool isVertex = (providerIndex == 4);   // PROVIDER_VERTEX
        bool isOllama = (providerIndex == 5);   // PROVIDER_OLLAMA

        if (vertexProjectInputField) vertexProjectInputField.transform.parent.gameObject.SetActive(isVertex);
        if (vertexLocationInputField) vertexLocationInputField.transform.parent.gameObject.SetActive(isVertex);
        if (vertexInfoText) vertexInfoText.gameObject.SetActive(isVertex);
        // API Key hidden for Vertex AI (uses gCloud CLI), visible for other providers
        if (apiKeyInputField) apiKeyInputField.transform.parent.gameObject.SetActive(!isVertex);
        if (ollamaHostInputField) ollamaHostInputField.transform.parent.gameObject.SetActive(isOllama);
        
        // Hide unused manual Client ID field and Use GCloud toggle UI, as gcloud is now forced
        if (vertexClientIdInputField) vertexClientIdInputField.transform.parent.gameObject.SetActive(false);
        if (vertexUseGcloudToggle) vertexUseGcloudToggle.transform.parent.gameObject.SetActive(false);
        
        // Hide unused manual Client ID field and Use GCloud toggle UI, as gcloud is now forced
        if (vertexClientIdInputField) vertexClientIdInputField.transform.parent.gameObject.SetActive(false);
        if (vertexUseGcloudToggle) vertexUseGcloudToggle.transform.parent.gameObject.SetActive(false);
        
        // Only show Auth button for desktop
#if UNITY_WEBGL && !UNITY_EDITOR
        if (vertexAuthButton) vertexAuthButton.gameObject.SetActive(false);
#else
        if (vertexAuthButton) vertexAuthButton.gameObject.SetActive(isVertex);
#endif
    }

    private void SaveSettings()
    {
        if (skipLoadingToggle) PlayerPrefs.SetInt(PREF_SKIP_LOADING, skipLoadingToggle.isOn ? 1 : 0);
        if (rightClickMenuToggle) PlayerPrefs.SetInt(PREF_RIGHT_CLICK_MENU, rightClickMenuToggle.isOn ? 1 : 0);
        if (desktopNotificationsToggle) PlayerPrefs.SetInt(PREF_DESKTOP_NOTIFICATIONS, desktopNotificationsToggle.isOn ? 1 : 0);
        if (lightweightModeToggle) PlayerPrefs.SetInt(PREF_LOW_SPEC_MODE, lightweightModeToggle.isOn ? 1 : 0);
        if (gazeTrackingToggle) PlayerPrefs.SetInt(PREF_GAZE_TRACKING, gazeTrackingToggle.isOn ? 1 : 0);
        PlayerPrefs.SetInt(PREF_LANGUAGE, languageDropdown ? languageDropdown.value : currentLanguageIndex);
        
        if (textSpeedSlider) PlayerPrefs.SetFloat(PREF_TEXT_SPEED, textSpeedSlider.value);
        if (textSpeedSlider) PlayerPrefs.SetFloat(PREF_TEXT_SPEED, textSpeedSlider.value);
        if (autoSpeedSlider) PlayerPrefs.SetFloat(PREF_AUTO_SPEED, autoSpeedSlider.value);
        if (autoModeToggle) PlayerPrefs.SetInt(PREF_AUTO_MODE, autoModeToggle.isOn ? 1 : 0); // Save Auto Mode

        if (masterVolumeSlider) PlayerPrefs.SetFloat(PREF_MASTER_VOL, masterVolumeSlider.value);
        if (bgmVolumeSlider) PlayerPrefs.SetFloat(PREF_BGM_VOL, bgmVolumeSlider.value);
        if (seVolumeSlider) PlayerPrefs.SetFloat(PREF_SE_VOL, seVolumeSlider.value);
        if (voiceVolumeSlider) PlayerPrefs.SetFloat(PREF_VOICE_VOL, voiceVolumeSlider.value);

        if (screenModeDropdown) PlayerPrefs.SetInt(PREF_SCREEN_MODE, screenModeDropdown.value);
        if (resolutionDropdown) PlayerPrefs.SetInt(PREF_RESOLUTION, resolutionDropdown.value);

        if (apiProviderDropdown) PlayerPrefs.SetInt(PREF_API_PROVIDER, apiProviderDropdown.value);
        
        // Save keys
        if (apiKeyInputField)
        {
            // Ensure current input is in buffer
            apiKeys[currentApiProviderIndex] = apiKeyInputField.text;
        }
        if (modelNameInputField)
        {
            // Ensure current input is in buffer
            modelNames[currentApiProviderIndex] = modelNameInputField.text;
        }

        foreach (var kvp in apiKeys)
        {
            SecurePrefs.SetProtectedString(PREF_API_KEY_PREFIX + kvp.Key, kvp.Value);
        }
        foreach (var kvp in modelNames)
        {
            PlayerPrefs.SetString(PREF_MODEL_NAME_PREFIX + kvp.Key, kvp.Value);
        }

        // Also update legacy key for compatibility if needed (optional, saving current)
        if (apiKeyInputField) SecurePrefs.SetProtectedString(PREF_API_KEY, apiKeyInputField.text);
        if (modelNameInputField) PlayerPrefs.SetString(PREF_MODEL_NAME, modelNameInputField.text);

        if (webSearchToggle) PlayerPrefs.SetInt(PREF_WEB_SEARCH, webSearchToggle.isOn ? 1 : 0);

        if (vertexProjectInputField) PlayerPrefs.SetString(PREF_VERTEX_PROJECT, vertexProjectInputField.text);
        if (vertexLocationInputField) PlayerPrefs.SetString(PREF_VERTEX_LOCATION, vertexLocationInputField.text);
        if (ollamaHostInputField) PlayerPrefs.SetString(PREF_OLLAMA_HOST, ollamaHostInputField.text);
        if (vertexClientIdInputField) PlayerPrefs.SetString(VertexOAuthService.PREF_VERTEX_CLIENT_ID, vertexClientIdInputField.text);
        if (vertexUseGcloudToggle) PlayerPrefs.SetInt("Config_VertexUseGcloud", vertexUseGcloudToggle.isOn ? 1 : 0);
        if (desktopBackgroundModeToggle) PlayerPrefs.SetInt(PREF_DESKTOP_BACKGROUND_MODE, desktopBackgroundModeToggle.isOn ? 1 : 0);

        // Save experimental settings
        SaveExperimentalSettings();

        PlayerPrefs.Save();
        ApplySettings(); // Apply immediately on save
        Debug.Log("Config Saved.");
    }

    private void ApplySettings()
    {
        ApplyPerformanceSettings();

        // Apply Screen Mode
        int screenModeIndex = PlayerPrefs.GetInt(PREF_SCREEN_MODE, 0);
        FullScreenMode mode = FullScreenMode.Windowed;
        
        string screenModeStr = "";
        if (screenModeDropdown != null && screenModeIndex >= 0 && screenModeIndex < screenModeDropdown.options.Count)
        {
            screenModeStr = screenModeDropdown.options[screenModeIndex].text.ToLower();
        }

        if (screenModeStr.Contains("window") || screenModeStr.Contains("ウィンドウ")) mode = FullScreenMode.Windowed;
        else if (screenModeStr.Contains("border") || screenModeStr.Contains("ボーダー")) mode = FullScreenMode.FullScreenWindow;
        else if (screenModeStr.Contains("full") || screenModeStr.Contains("フル")) mode = FullScreenMode.FullScreenWindow;
        else
        {
            // Fallback assuming the typical order 1. FullScreen 2. Windowed 3. Borderless
            switch (screenModeIndex)
            {
                case 0: mode = FullScreenMode.FullScreenWindow; break;
                case 1: mode = FullScreenMode.Windowed; break;
                case 2: mode = FullScreenMode.FullScreenWindow; break;
            }
        }
        
        // Apply Resolution
        int resIndex = PlayerPrefs.GetInt(PREF_RESOLUTION, 0);
        int width = 1920;
        int height = 1080;

        string resStr = "";
        if (resolutionDropdown != null && resIndex >= 0 && resIndex < resolutionDropdown.options.Count)
        {
            resStr = resolutionDropdown.options[resIndex].text;
        }

        if (resStr.Contains("1920")) { width = 1920; height = 1080; }
        else if (resStr.Contains("1600")) { width = 1600; height = 900; }
        else if (resStr.Contains("1280")) { width = 1280; height = 720; }
        else if (resStr.Contains("854")) { width = 854; height = 480; }

        Screen.SetResolution(width, height, mode);
    }

    private void OnSaveClicked()
    {
        SaveSettings();
        ApplyConfigLanguage(languageDropdown ? languageDropdown.value : currentLanguageIndex);
        Hide();
        onCloseCallback?.Invoke();
    }

    private void OnCancelClicked()
    {
        Hide();
        onCloseCallback?.Invoke();
    }

    private IEnumerator FadeCanvas(float start, float end, bool disableOnFinish = false, bool restoreOutlines = false)
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
        if (restoreOutlines) UpdateSidebarVisuals(); // Re-enable active outline after fade
    }

    private void SetupLanguageDropdown()
    {
        if (languageDropdown == null)
        {
            var pageGeneral = (categoryPages != null && categoryPages.Count > 0) ? categoryPages[0] : null;
            if (pageGeneral != null)
            {
                var dropdownTf = pageGeneral.transform.Find("Row_DisplayLanguage/Dropdown_DisplayLanguage");
                if (dropdownTf != null) languageDropdown = dropdownTf.GetComponent<TMP_Dropdown>();

                var labelTf = pageGeneral.transform.Find("Row_DisplayLanguage/Label");
                if (labelTf != null) languageLabelText = labelTf.GetComponent<TextMeshProUGUI>();
            }
        }

        if (languageDropdown == null)
        {
            Debug.LogWarning("ConfigPanelController: languageDropdown is not assigned. Place Row_DisplayLanguage in ConfigPanel/Page_General and assign references in inspector.");
            return;
        }

        languageDropdown.onValueChanged.RemoveListener(OnLanguageDropdownChanged);
        currentLanguageIndex = PlayerPrefs.GetInt(PREF_LANGUAGE, 0);
        RefreshLanguageDropdownOptions();
        languageDropdown.SetValueWithoutNotify(currentLanguageIndex);
        languageDropdown.onValueChanged.AddListener(OnLanguageDropdownChanged);
        ApplyConfigLanguage(currentLanguageIndex);
    }

    private void OnLanguageDropdownChanged(int index)
    {
        currentLanguageIndex = index;
        ApplyConfigLanguage(index);
    }

    private void RefreshLanguageDropdownOptions()
    {
        if (languageDropdown == null) return;

        int keepIndex = Mathf.Clamp(currentLanguageIndex, 0, 7);
        languageDropdown.ClearOptions();
        languageDropdown.AddOptions(new List<string>
        {
            "日本語",
            "English",
            "中文",
            "한국어",
            "Español",
            "Français",
            "Deutsch",
            "Русский"
        });
        languageDropdown.SetValueWithoutNotify(keepIndex);
    }

    private string GetLocalizedCategoryHeader(int index)
    {
        switch (index)
        {
            case 0: return Localize("システム", "System", "系统", "시스템", "Sistema", "Système", "System", "Система");
            case 1: return Localize("テキスト", "Text", "文本", "텍스트", "Texto", "Texte", "Text", "Текст");
            case 2: return Localize("サウンド", "Sound", "音频", "사운드", "Sonido", "Son", "Sound", "Звук");
            case 3: return Localize("グラフィック", "Graphics", "图像", "그래픽", "Gráficos", "Graphismes", "Grafik", "Графика");
            case 4: return Localize("API", "API", "API", "API", "API", "API", "API", "API");
            default: return Localize("試験的機能", "Experimental", "试验性功能", "실험적 기능", "Experimental", "Expérimental", "Experimentell", "Эксперимент");
        }
    }

    private void ApplyConfigLanguage(int languageIndex)
    {
        currentLanguageIndex = Mathf.Clamp(languageIndex, 0, 7);
        bool toEnglish = currentLanguageIndex != 0;

        RefreshLanguageDropdownOptions();

        if (languageLabelText != null)
        {
            languageLabelText.text = GetSpacingTaggedText(Localize("表示言語", "Display Language", "显示语言", "표시 언어", "Idioma", "Langue", "Sprache", "Язык"));
        }

        if (headerText != null)
        {
            headerText.text = GetLocalizedCategoryHeader(activeCategoryIndex);
        }

        UpdateCategoryButtonTexts(currentLanguageIndex);
        UpdateCommonButtonTexts(currentLanguageIndex);
        UpdateConfigLabelTexts(currentLanguageIndex);

        // Notify other panels to refresh their language-dependent strings (names, etc)
        var chat = FindObjectOfType<AmadeusChatController>(true);
        if (chat != null) chat.UpdateLanguage();

        var backLog = FindObjectOfType<BackLogController>(true);
        if (backLog != null) backLog.UpdateLanguage();

        var changeLog = FindObjectOfType<ChangeLogPanelController>(true);
        if (changeLog != null) changeLog.UpdateLanguage();

        var help = FindObjectOfType<HelpPanelController>(true);
        if (help != null) help.UpdateLanguage();

        var status = FindObjectOfType<StatusPanelController>(true);
        if (status != null) status.UpdateLanguage();
    }

    private void UpdateCategoryButtonTexts(int languageIndex)
    {
        if (categoryButtons == null) return;

        string[] ja = { "システム", "テキスト", "サウンド", "グラフィック", "API", "試験的機能" };
        string[] en = { "System", "Text", "Sound", "Graphics", "API", "Experimental" };
        string[] zh = { "系统", "文本", "音频", "图像", "API", "试验性功能" };
        string[] ko = { "시스템", "텍스트", "사운드", "그래픽", "API", "실험적 기능" };
        string[] es = { "Sistema", "Texto", "Sonido", "Gráficos", "API", "Experimental" };
        string[] fr = { "Système", "Texte", "Son", "Graphismes", "API", "Expérimental" };
        string[] de = { "System", "Text", "Sound", "Grafik", "API", "Experimentell" };
        string[] ru = { "Система", "Текст", "Звук", "Графика", "API", "Эксперимент" };

        for (int i = 0; i < categoryButtons.Count && i < ja.Length; i++)
        {
            if (categoryButtons[i] == null) continue;
            var tmp = categoryButtons[i].GetComponentInChildren<TextMeshProUGUI>(true);
            if (tmp != null)
            {
                switch (languageIndex)
                {
                    case 0: tmp.text = ja[i]; break;
                    case 1: tmp.text = en[i]; break;
                    case 2: tmp.text = zh[i]; break;
                    case 3: tmp.text = ko[i]; break;
                    case 4: tmp.text = es[i]; break;
                    case 5: tmp.text = fr[i]; break;
                    case 6: tmp.text = de[i]; break;
                    case 7: tmp.text = GetSpacingTaggedText(ru[i]); break;
                    default: tmp.text = en[i]; break;
                }
            }
        }
    }

    private void UpdateCommonButtonTexts(int languageIndex)
    {
        if (saveButton != null)
        {
            var tmp = saveButton.GetComponentInChildren<TextMeshProUGUI>(true);
            if (tmp != null) tmp.text = GetSpacingTaggedText(Localize("適用", "Apply", "应用", "적용", "Aplicar", "Appliquer", "Anwenden", "Применить"));
        }

        if (cancelButton != null)
        {
            var tmp = cancelButton.GetComponentInChildren<TextMeshProUGUI>(true);
            if (tmp != null) tmp.text = GetSpacingTaggedText(Localize("キャンセル", "Cancel", "取消", "취소", "Cancelar", "Annuler", "Abbrechen", "Отмена"));
        }
    }

    private string Localize(string ja, string en, string zh, string ko, string es, string fr, string de, string ru)
    {
        switch (currentLanguageIndex)
        {
            case 0: return ja;
            case 1: return en;
            case 2: return zh;
            case 3: return ko;
            case 4: return es;
            case 5: return fr;
            case 6: return de;
            case 7: return ru;
            default: return en;
        }
    }

    private void UpdateConfigLabelTexts(int index)
    {
        var map = new System.Collections.Generic.List<string[]>
        {
            new string[] { "起動画面スキップ", "Skip Loading Screen", "跳过启动画面", "로딩 화면 스킵", "Saltar pantalla de carga", "Passer l'écran de chargement", "Ladebildschirm überspringen", "Пропускать экран загрузки" },
            new string[] { "右クリックメニュー", "Right Click Menu", "右键菜单", "우클릭 메뉴", "Menú clic derecho", "Menu clic droit", "Rechtsklick-Menü", "Меню правой кнопки мыши" },
            new string[] { "通知を表示", "Show Notifications", "显示通知", "알림 표시", "Mostrar notificaciones", "Afficher les notifications", "Benachrichtigungen anzeigen", "Показывать уведомления" },
            new string[] { "軽量化モード", "Lightweight Mode", "轻量模式", "경량 모드", "Modo ligero", "Mode léger", "Leichtmodus", "Легкий режим" },
            new string[] { "視線トラッキング", "Eye Tracking", "视线追踪", "시선 추적", "Seguimiento de ojos", "Suivi oculaire", "Blick-Tracking", "Трекинг взгляда" },
            new string[] { "文字表示速度", "Text Speed", "文字速度", "텍스트 속도", "Velocidad de texto", "Vitesse du texte", "Textgeschwindigkeit", "Скорость текста" },
            new string[] { "オート表示", "Auto Mode", "自动模式", "자동 모드", "Modo automático", "Mode automatique", "Auto-Modus", "Авторежим" },
            new string[] { "オート待機時間", "Auto Wait Time", "自动等待时间", "자동 대기 시간", "Tiempo de espera", "Temps d'attente", "Wartezeit", "Время ожидания" },
            new string[] { "マスター音量", "Master Volume", "主音量", "마스터 볼륨", "Volumen maestro", "Volume principal", "Hauptlautstärke", "Громкость" },
            new string[] { "BGM音量", "BGM Volume", "BGM音量", "BGM 볼륨", "Volumen BGM", "Volume BGM", "BGM-Lautstärke", "BGM" },
            new string[] { "SE音量", "SE Volume", "音效音量", "SE 볼륨", "Volumen efectos", "Volume effets", "Effekte", "SE" },
            new string[] { "ボイス音量", "Voice Volume", "语音音量", "음성 볼륨", "Volumen de voz", "Volume voix", "Stimme", "Голос" },
            new string[] { "画面モード", "Screen Mode", "屏幕模式", "화면 모드", "Modo de pantalla", "Mode d'écran", "Bildschirmmodus", "Экран" },
            new string[] { "解像度", "Resolution", "分辨率", "해상도", "Resolución", "Résolution", "Auflösung", "Разрешение" },
            new string[] { "LLM APIプロバイダ", "LLM API Provider", "LLM提供商", "LLM 제공자", "Proveedor LLM", "Fournisseur LLM", "LLM-Anbieter", "Провайдер" },
            new string[] { "LLM モデル名", "LLM Model Name", "模型名称", "모델명", "Nombre del modelo", "Nom du modèle", "Modellname", "Имя модели" },
            new string[] { "LLM Web検索", "LLM Web Search", "网络搜索", "웹 검색", "Búsqueda web", "Recherche web", "Websuche", "Веб-поиск" },
            new string[] { "※ Vertexを使用するためには、gCloud CLIのインストールが必要です。", "* gCloud CLI is required to use Vertex.", "* 使用Vertex需要安装gCloud CLI。", "* Vertex를 사용하려면 gCloud CLI가 필요합니다。", "* Se requiere gCloud CLI.", "* gCloud CLI est requis.", "* gCloud CLI wird benötigt.", "* Требуется gCloud CLI." },
            new string[] { "TTS (音声合成)", "TTS (Speech)", "TTS (语音合成)", "TTS (음성 합성)", "TTS", "TTS", "TTS", "TTS (Синтез)" },
            new string[] { "STT (音声認識)", "STT (Dictation)", "STT (语音识别)", "STT (음성 인식)", "STT", "STT", "STT", "STT (Распознавание)" },
            new string[] { "知識ベース (RAG)", "Knowledge Base (RAG)", "知识库 (RAG)", "지식 베이스 (RAG)", "Base de conocimientos (RAG)", "Base de connaissances (RAG)", "Wissensdatenbank (RAG)", "База знаний (RAG)" },
            new string[] { "デスクトップ背景モード", "Desktop Background Mode", "桌面背景模式", "데스크톱 배경 모드", "Modo de fondo de escritorio", "Mode d'arrière-plan du bureau", "Desktop-Hintergrundmodus", "Режим фона рабочего стола" },
            // Native experimental labels
            new string[] { "ネイティブTTS有効化", "Enable Native TTS", "启用原生TTS", "네이티브 TTS 활성화", "Habilitar TTS nativo", "Activer TTS natif", "Nativen TTS aktivieren", "Включить нативный TTS" },
            new string[] { "TTS GPU推論", "TTS GPU Inference", "TTS GPU推理", "TTS GPU 추론", "Inferencia GPU TTS", "Inférence GPU TTS", "TTS-GPU-Inferenz", "TTS GPU-инференс" },
            new string[] { "ネイティブSTT有効化", "Enable Native STT", "启用原生STT", "네이티브 STT 활성화", "Habilitar STT nativo", "Activer STT natif", "Nativen STT aktivieren", "Включить нативный STT" },
            new string[] { "STT GPU推論", "STT GPU Inference", "STT GPU推理", "STT GPU 추론", "Inferencia GPU STT", "Inférence GPU STT", "STT-GPU-Inferenz", "STT GPU-инференс" },
            new string[] { "ネイティブSTT言語", "Native STT Language", "原生STT语言", "네이티브 STT 언어", "Idioma STT nativo", "Langue STT native", "Native STT-Sprache", "Язык нативного STT" },
            new string[] { "ネイティブRAG有効化", "Enable Native RAG", "启用原生RAG", "네이티브 RAG 활성화", "Habilitar RAG nativo", "Activer RAG natif", "Nativen RAG aktivieren", "Включить нативный RAG" },
            new string[] { "ネイティブRAG上位件数", "Native RAG Top-K", "原生RAG排名前K条", "네이티브 RAG 상위 K", "Top-K de RAG nativo", "Top-K du RAG natif", "Nativer RAG Top-K", "Топ-K нативного RAG" },
            new string[] { "ネイティブRAG類似度閾値", "Native RAG Threshold", "原生RAG相似度阈值", "네이티브 RAG 유사도 임계값", "Umbral de RAG nativo", "Seuil de RAG natif", "Nativer RAG-Schwelle", "Порог нативного RAG" },
            // Provider URL labels
            new string[] { "Ollama Host", "Ollama Host", "Ollama主机", "Ollama 호스트", "Host de Ollama", "Hôte Ollama", "Ollama-Host", "Ollama хост" }
        };

        var texts = GetComponentsInChildren<TextMeshProUGUI>(true);
        foreach (var tmp in texts)
        {
            if (tmp == null || string.IsNullOrEmpty(tmp.text)) continue;

            string tText = tmp.text.Trim();
            // Strip TMP rich-text tags (e.g. <cspace>) so we can match against plain mapping strings
            string strippedText = System.Text.RegularExpressions.Regex.Replace(tText, @"<[^>]+>", "").Trim();
            foreach (var mapping in map)
            {
                bool found = false;
                foreach (var str in mapping)
                {
                    if (strippedText == str)
                    {
                        found = true;
                        break;
                    }
                }
                if (found)
                {
                    tmp.text = GetSpacingTaggedText(mapping[Mathf.Clamp(index, 0, 7)]);
                    break;
                }
            }
        }

        if (screenModeDropdown != null)
        {
            int idx = screenModeDropdown.value;
            screenModeDropdown.ClearOptions();
            var options = new List<string> { "Fullscreen", "Windowed", "Borderless" };
            
            options = new List<string> {
                Localize("フルスクリーン", "Fullscreen", "全屏", "전체 화면", "Pantalla completa", "Plein écran", "Vollbild", "Полный экран"),
                Localize("ウィンドウ", "Windowed", "窗口模式", "창 모드", "Modo ventana", "Mode fenêtré", "Fenstermodus", "Оконный режим"),
                "Borderless" // Keep borderless but as is
            };
            
            screenModeDropdown.AddOptions(options);
            screenModeDropdown.SetValueWithoutNotify(Mathf.Clamp(idx, 0, screenModeDropdown.options.Count - 1));
        }
    }

    private static int GetIntWithLegacy(string primaryKey, string legacyKey, int defaultValue)
    {
        if (PlayerPrefs.HasKey(primaryKey))
        {
            return PlayerPrefs.GetInt(primaryKey, defaultValue);
        }
        return PlayerPrefs.GetInt(legacyKey, defaultValue);
    }

    private static string GetStringWithLegacy(string primaryKey, string legacyKey, string defaultValue)
    {
        if (PlayerPrefs.HasKey(primaryKey))
        {
            return PlayerPrefs.GetString(primaryKey, defaultValue);
        }
        return PlayerPrefs.GetString(legacyKey, defaultValue);
    }

    private static void ApplyPerformanceSettings()
    {
        bool lowSpec = PlayerPrefs.GetInt(PREF_LOW_SPEC_MODE, 0) == 1;
        Application.targetFrameRate = lowSpec ? 30 : 60;
        QualitySettings.vSyncCount = 0;
        AudioManager.Instance?.ApplyPerformanceMode(lowSpec);
    }
    
    private void SaveExperimentalSettings()
    {
        if (toolsEnabledToggle) PlayerPrefs.SetInt(PREF_TOOLS_ENABLED, toolsEnabledToggle.isOn ? 1 : 0);

        if (nativeTtsEnabledToggle) PlayerPrefs.SetInt(PREF_NATIVE_TTS_ENABLED, nativeTtsEnabledToggle.isOn ? 1 : 0);
        if (nativeTtsGpuToggle) PlayerPrefs.SetInt(PREF_NATIVE_TTS_GPU, nativeTtsGpuToggle.isOn ? 1 : 0);
        if (nativeSttEnabledToggle) PlayerPrefs.SetInt(PREF_NATIVE_STT_ENABLED, nativeSttEnabledToggle.isOn ? 1 : 0);
        if (nativeSttGpuToggle) PlayerPrefs.SetInt(PREF_NATIVE_STT_GPU, nativeSttGpuToggle.isOn ? 1 : 0);
        if (nativeSttLanguageInput) PlayerPrefs.SetString(PREF_NATIVE_STT_LANGUAGE, nativeSttLanguageInput.text);
        if (nativeRagEnabledToggle) PlayerPrefs.SetInt(PREF_NATIVE_RAG_ENABLED, nativeRagEnabledToggle.isOn ? 1 : 0);
        if (nativeRagTopKInput)
        {
            if (int.TryParse(nativeRagTopKInput.text, out int topK))
                PlayerPrefs.SetInt(PREF_NATIVE_RAG_TOPK, Mathf.Clamp(topK, 1, 20));
        }
        if (nativeRagThresholdInput)
        {
            if (float.TryParse(nativeRagThresholdInput.text, out float threshold))
                PlayerPrefs.SetFloat(PREF_NATIVE_RAG_THRESHOLD, Mathf.Clamp(threshold, 0f, 1f));
        }
    }

    private string GetSpacingTaggedText(string text)
    {
        if (currentLanguageIndex != 7 || string.IsNullOrEmpty(text)) return text;
        return System.Text.RegularExpressions.Regex.Replace(text, @"([\u0400-\u04FF]+)", "<cspace=-8.4px>$1</cspace>");
    }
}
