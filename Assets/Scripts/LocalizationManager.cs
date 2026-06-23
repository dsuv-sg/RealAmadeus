using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

/// <summary>
/// Centralized localization manager matching the QT version (V1.3Q).
/// Loads translation JSON files from Resources/locales/ at startup
/// and exposes a key-based lookup API plus an event when the language changes.
///
/// Supported language codes (also matching QT's order):
///   0: ja  1: en  2: zh  3: ko  4: es  5: fr  6: de  7: ru
///   8: uk  9: pt  10: tr
/// </summary>
public class LocalizationManager : MonoBehaviour
{
    private static LocalizationManager _instance;
    public static LocalizationManager Instance
    {
        get
        {
            if (_instance == null)
            {
                _instance = FindObjectOfType<LocalizationManager>();
                if (_instance == null)
                {
                    var go = new GameObject("[LocalizationManager]");
                    if (Application.isPlaying)
                    {
                        DontDestroyOnLoad(go);
                    }
                    _instance = go.AddComponent<LocalizationManager>();
                }
            }
            return _instance;
        }
        private set
        {
            _instance = value;
        }
    }

    public const string PREF_LANGUAGE = "Config_Language";

    public const int LANG_JA = 0;
    public const int LANG_EN = 1;
    public const int LANG_ZH = 2;
    public const int LANG_KO = 3;
    public const int LANG_ES = 4;
    public const int LANG_FR = 5;
    public const int LANG_DE = 6;
    public const int LANG_RU = 7;
    public const int LANG_UK = 8;
    public const int LANG_PT = 9;
    public const int LANG_TR = 10;

    public static readonly string[] LanguageCodes = { "ja", "en", "zh", "ko", "es", "fr", "de", "ru", "uk", "pt", "tr" };
    public static readonly string[] LanguageDisplayNames = {
        "日本語", "English", "中文", "한국어", "Español",
        "Français", "Deutsch", "Русский",
        "Українська", "Português", "Türkçe"
    };

    [SerializeField] private int currentLanguageIndex = 0;
    public int CurrentLanguageIndex => currentLanguageIndex;

    private readonly Dictionary<string, Dictionary<string, string>> _translations = new Dictionary<string, Dictionary<string, string>>();

    public event Action OnTranslationsChanged;
    public event Action<int> OnLanguageChanged;

    public static LocalizationManager GetOrCreate()
    {
        return Instance;
    }

    void Awake()
    {
        if (_instance != null && _instance != this)
        {
            Destroy(gameObject);
            return;
        }
        _instance = this;
        if (Application.isPlaying)
        {
            DontDestroyOnLoad(gameObject);
        }
        LoadAllLocales();
        currentLanguageIndex = Mathf.Clamp(PlayerPrefs.GetInt(PREF_LANGUAGE, 0), 0, LanguageCodes.Length - 1);
    }

    private void LoadAllLocales()
    {
        _translations.Clear();
        foreach (var code in LanguageCodes)
        {
            string json = null;

            // 1. External path next to executable (allows runtime edits by translators, like QT version)
            try
            {
                string appDir = Directory.GetParent(Application.dataPath).FullName;
                string externalPath = Path.Combine(appDir, "resources", "locales", code + ".json");
                if (File.Exists(externalPath))
                {
                    json = File.ReadAllText(externalPath);
                }
            }
            catch (Exception ex) { }

            // 2. StreamingAssets path
            if (string.IsNullOrEmpty(json))
            {
                try
                {
                    string streamingPath = Path.Combine(Application.streamingAssetsPath, "locales", code + ".json");
                    if (File.Exists(streamingPath))
                    {
                        json = File.ReadAllText(streamingPath);
                    }
                }
                catch (Exception ex) { }
            }

            // 3. Editor path
#if UNITY_EDITOR
            if (string.IsNullOrEmpty(json))
            {
                try
                {
                    string editorPath = Path.Combine(Application.dataPath, "Resources", "locales", code + ".json");
                    if (File.Exists(editorPath))
                    {
                        json = File.ReadAllText(editorPath);
                    }
                }
                catch (Exception ex) { }
            }
#endif

            // 4. Resources fallback
            if (string.IsNullOrEmpty(json))
            {
                TextAsset asset = null;
                try { asset = Resources.Load<TextAsset>("locales/" + code); }
                catch (Exception ex) { Debug.LogWarning("[Localization] Resources.Load failed for " + code + ": " + ex.Message); }
                if (asset != null) json = asset.text;
            }

            if (string.IsNullOrEmpty(json))
            {
                _translations[code] = new Dictionary<string, string>();
                continue;
            }

            try
            {
                var dict = MiniJson.Deserialize(json) as Dictionary<string, object>;
                var translated = new Dictionary<string, string>();
                if (dict != null)
                {
                    foreach (var kvp in dict)
                    {
                        if (kvp.Value != null) translated[kvp.Key] = kvp.Value.ToString();
                    }
                }
                _translations[code] = translated;
            }
            catch (Exception ex)
            {
                Debug.LogWarning("[Localization] JSON parse failed for " + code + ": " + ex.Message);
                _translations[code] = new Dictionary<string, string>();
            }
        }

        _valueToKey.Clear();
        foreach (var code in new string[] { "en", "ja" })
        {
            if (_translations.TryGetValue(code, out var dict))
            {
                foreach (var kvp in dict)
                {
                    string val = kvp.Value.Replace("\r\n", "\n").Replace("\r", "\n").Trim();
                    val = System.Text.RegularExpressions.Regex.Replace(val, @"<[^>]+>", "").Trim();
                    if (!string.IsNullOrEmpty(val) && val.Length > 1)
                    {
                        _valueToKey[val] = kvp.Key;
                    }
                }
            }
        }
    }

    private readonly Dictionary<string, string> _valueToKey = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

    public string LookupKey(string value)
    {
        if (string.IsNullOrEmpty(value)) return null;
        string cleaned = value.Replace("\r\n", "\n").Replace("\r", "\n").Trim();
        cleaned = System.Text.RegularExpressions.Regex.Replace(cleaned, @"<[^>]+>", "").Trim();
        if (_valueToKey.TryGetValue(cleaned, out string key)) return key;
        return null;
    }

    /// <summary>
    /// Translate a key for the currently active language.
    /// Returns the supplied fallback if the key is missing in the locale.
    /// </summary>
    public string T(string key, string fallback = null)
    {
        if (string.IsNullOrEmpty(key)) return fallback ?? string.Empty;
        string code = LanguageCodes[Mathf.Clamp(currentLanguageIndex, 0, LanguageCodes.Length - 1)];
        if (_translations.TryGetValue(code, out var dict) && dict.TryGetValue(key, out var value))
        {
            return value;
        }
        if (currentLanguageIndex != LANG_EN && _translations.TryGetValue("en", out var enDict) && enDict.TryGetValue(key, out var enVal))
        {
            return enVal;
        }
        return fallback ?? key;
    }

    public string TFormat(string key, string fallback, params object[] args)
    {
        string template = T(key, fallback);
        if (template == null) return string.Empty;
        try { return string.Format(template, args); }
        catch { return template; }
    }

    /// <summary>
    /// Resolve a key for a specific language index (regardless of current selection).
    /// Used by panels that need to render other-language previews or fallbacks.
    /// </summary>
    public string TForLanguage(int langIndex, string key, string fallback = null)
    {
        if (string.IsNullOrEmpty(key)) return fallback ?? string.Empty;
        if (langIndex < 0 || langIndex >= LanguageCodes.Length) return fallback ?? key;
        string code = LanguageCodes[langIndex];
        if (_translations.TryGetValue(code, out var dict) && dict.TryGetValue(key, out var value))
        {
            return value;
        }
        if (langIndex != LANG_EN && _translations.TryGetValue("en", out var enDict) && enDict.TryGetValue(key, out var enVal))
        {
            return enVal;
        }
        return fallback ?? key;
    }

    public void SetLanguage(int langIndex)
    {
        langIndex = Mathf.Clamp(langIndex, 0, LanguageCodes.Length - 1);
        if (langIndex == currentLanguageIndex)
        {
            LoadAllLocales(); // Reload translations to fetch any runtime edits
            OnLanguageChanged?.Invoke(langIndex);
            OnTranslationsChanged?.Invoke();
            return;
        }
        currentLanguageIndex = langIndex;
        PlayerPrefs.SetInt(PREF_LANGUAGE, langIndex);
        PlayerPrefs.Save();
        LoadAllLocales(); // Reload translations to fetch any runtime edits
        OnLanguageChanged?.Invoke(langIndex);
        OnTranslationsChanged?.Invoke();
    }

    public void NotifyChanged()
    {
        OnTranslationsChanged?.Invoke();
    }

    public bool HasKey(string key)
    {
        if (string.IsNullOrEmpty(key)) return false;
        string code = LanguageCodes[Mathf.Clamp(currentLanguageIndex, 0, LanguageCodes.Length - 1)];
        return _translations.TryGetValue(code, out var dict) && dict.ContainsKey(key);
    }

    public string GetLanguageCode(int langIndex = -1)
    {
        if (langIndex < 0) langIndex = currentLanguageIndex;
        return LanguageCodes[Mathf.Clamp(langIndex, 0, LanguageCodes.Length - 1)];
    }

    /// <summary>Returns true when the language uses Cyrillic script (Russian/Ukrainian).</summary>
    public bool IsCyrillic(int langIndex = -1)
    {
        if (langIndex < 0) langIndex = currentLanguageIndex;
        return langIndex == LANG_RU || langIndex == LANG_UK;
    }

    /// <summary>Returns true when the language is Ukrainian (which needs tighter Cyrillic_I spacing).</summary>
    public bool IsUkrainian(int langIndex = -1)
    {
        if (langIndex < 0) langIndex = currentLanguageIndex;
        return langIndex == LANG_UK;
    }

    public bool IsCyrillicI(char c)
    {
        int code = c;
        return code == 0x0406 || code == 0x0456 || code == 0x0407 || code == 0x0457;
    }

    public string GetSpacingTaggedText(string text, int lang = -1)
    {
        if (lang < 0) lang = currentLanguageIndex;
        if (string.IsNullOrEmpty(text) || !IsCyrillic(lang)) return text;

        var result = new System.Text.StringBuilder();
        string currentType = "";
        var currentText = new System.Text.StringBuilder();

        System.Action flush = () => {
            if (currentText.Length == 0) return;
            string run = currentText.ToString();
            if (currentType == "cyrillic")
            {
                // Align with Qt version (QML spacing is -7.0px for both Russian and Ukrainian)
                string spacing = "-7.0px";
                result.Append("<cspace=").Append(spacing).Append(">").Append(run).Append("</cspace>");
            }
            else if (currentType == "cyrillic_i")
            {
                // Align with Qt version (QML spacing is -2.0px for both Russian and Ukrainian)
                string spacing = "-2.0px";
                result.Append("<cspace=").Append(spacing).Append(">").Append(run).Append("</cspace>");
            }
            else
            {
                result.Append(run);
            }
            currentText.Clear();
        };

        for (int i = 0; i < text.Length; i++)
        {
            char c = text[i];
            string type = "other";

            if (c >= 0x0400 && c <= 0x04FF)
            {
                if (IsCyrillicI(c))
                {
                    type = "cyrillic_i";
                }
                else if (i + 1 < text.Length && IsCyrillicI(text[i + 1]))
                {
                    type = "cyrillic_i";
                }
                else if (i - 1 >= 0 && IsCyrillicI(text[i - 1]))
                {
                    type = "cyrillic_i";
                }
                else
                {
                    type = "cyrillic";
                }
            }

            if (type != currentType && currentText.Length > 0)
            {
                flush();
            }
            if (currentText.Length == 0) currentType = type;
            currentText.Append(c);
        }
        flush();

        return result.ToString();
    }
}
