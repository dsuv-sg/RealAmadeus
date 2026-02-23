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

    private Coroutine currentFadeCoroutine;
    private Action onCloseCallback;
    private int activeCategoryIndex = 0;
    
    // Multiple API Key and Model Name Support
    private Dictionary<int, string> apiKeys = new Dictionary<int, string>();
    private Dictionary<int, string> modelNames = new Dictionary<int, string>();
    private int currentApiProviderIndex = 0;
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
        if (textSpeedSlider) textSpeedSlider.onValueChanged.AddListener(val => UpdateSliderText(textSpeedValueText, val, "x{0:F1}"));
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
                apiProviderDropdown.AddOptions(new List<string> { "OpenAI", "Google Gemini", "Anthropic Claude", "Groq", "Vertex AI" });
            }
            else
            {
                // Ensure Vertex AI exists
                bool hasVertex = false;
                foreach (var opt in apiProviderDropdown.options)
                {
                    if (opt.text == "Vertex AI")
                    {
                        hasVertex = true;
                        break;
                    }
                }
                if (!hasVertex)
                {
                    apiProviderDropdown.options.Add(new TMP_Dropdown.OptionData("Vertex AI"));
                }
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

        ApplySettings(); // Apply settings on startup

        if (vertexAuthButton) vertexAuthButton.onClick.AddListener(OnVertexAuthClicked);
        if (vertexUseGcloudToggle) vertexUseGcloudToggle.onValueChanged.AddListener((val) => UpdateVertexFieldsVisibility(currentApiProviderIndex));
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
            string[] headers = { "基本設定", "テキスト設定", "サウンド設定", "グラフィック設定", "API設定" };
            if (index >= 0 && index < headers.Length) headerText.text = headers[index];
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
        // Add safe checks and defaults
        if (skipLoadingToggle) skipLoadingToggle.isOn = PlayerPrefs.GetInt(PREF_SKIP_LOADING, 0) == 1;
        if (rightClickMenuToggle) rightClickMenuToggle.isOn = PlayerPrefs.GetInt(PREF_RIGHT_CLICK_MENU, 1) == 1;
        
        if (textSpeedSlider) { textSpeedSlider.value = PlayerPrefs.GetFloat(PREF_TEXT_SPEED, 1.0f); UpdateSliderText(textSpeedValueText, textSpeedSlider.value, "x{0:F1}"); }
        if (textSpeedSlider) { textSpeedSlider.value = PlayerPrefs.GetFloat(PREF_TEXT_SPEED, 1.0f); UpdateSliderText(textSpeedValueText, textSpeedSlider.value, "x{0:F1}"); }
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
                string key = PlayerPrefs.GetString(PREF_API_KEY_PREFIX + i, "");
                string modelName = PlayerPrefs.GetString(PREF_MODEL_NAME_PREFIX + i, "");
                
                // Legacy Migration: If Groq (3) is explicitly empty, check old global key
                // Or just try to migrate global key if specific key is empty
                if (string.IsNullOrEmpty(key) && i == currentApiProviderIndex)
                {
                     string legacyKey = PlayerPrefs.GetString(PREF_API_KEY, "");
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
        if (vertexLocationInputField) vertexLocationInputField.text = PlayerPrefs.GetString(PREF_VERTEX_LOCATION, "global");
        if (vertexClientIdInputField) vertexClientIdInputField.text = PlayerPrefs.GetString(VertexOAuthService.PREF_VERTEX_CLIENT_ID, "");
        if (vertexUseGcloudToggle) vertexUseGcloudToggle.isOn = PlayerPrefs.GetInt("Config_VertexUseGcloud", 0) == 1;

        UpdateVertexFieldsVisibility(currentApiProviderIndex);
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

        UpdateVertexFieldsVisibility(newIndex);
    }

    /// <summary>
    /// Shows Vertex AI fields only when Vertex AI provider (index 4) is selected.
    /// </summary>
    private void UpdateVertexFieldsVisibility(int providerIndex)
    {
        bool isVertex = (providerIndex == 4); // PROVIDER_VERTEX
        if (vertexProjectInputField) vertexProjectInputField.transform.parent.gameObject.SetActive(isVertex);
        if (vertexLocationInputField) vertexLocationInputField.transform.parent.gameObject.SetActive(isVertex);
        if (vertexInfoText) vertexInfoText.gameObject.SetActive(isVertex);
        if (apiKeyInputField) apiKeyInputField.transform.parent.gameObject.SetActive(!isVertex);
        
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
            PlayerPrefs.SetString(PREF_API_KEY_PREFIX + kvp.Key, kvp.Value);
        }
        foreach (var kvp in modelNames)
        {
            PlayerPrefs.SetString(PREF_MODEL_NAME_PREFIX + kvp.Key, kvp.Value);
        }

        // Also update legacy key for compatibility if needed (optional, saving current)
        if (apiKeyInputField) PlayerPrefs.SetString(PREF_API_KEY, apiKeyInputField.text);
        if (modelNameInputField) PlayerPrefs.SetString(PREF_MODEL_NAME, modelNameInputField.text);

        if (webSearchToggle) PlayerPrefs.SetInt(PREF_WEB_SEARCH, webSearchToggle.isOn ? 1 : 0);

        if (vertexProjectInputField) PlayerPrefs.SetString(PREF_VERTEX_PROJECT, vertexProjectInputField.text);
        if (vertexLocationInputField) PlayerPrefs.SetString(PREF_VERTEX_LOCATION, vertexLocationInputField.text);
        if (vertexClientIdInputField) PlayerPrefs.SetString(VertexOAuthService.PREF_VERTEX_CLIENT_ID, vertexClientIdInputField.text);
        if (vertexUseGcloudToggle) PlayerPrefs.SetInt("Config_VertexUseGcloud", vertexUseGcloudToggle.isOn ? 1 : 0);

        PlayerPrefs.Save();
        ApplySettings(); // Apply immediately on save
        Debug.Log("Config Saved.");
    }

    private void ApplySettings()
    {
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
}
