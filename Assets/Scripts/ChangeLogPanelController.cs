using System;
using System.Collections;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.InputSystem;
using TMPro;

public class ChangeLogPanelController : MonoBehaviour
{
    [Header("UI References")]
    public CanvasGroup panelCanvasGroup;
    public Button closeButton;
    public ScrollRect contentScrollRect;

    [Header("Settings")]
    public float fadeDuration = 0.3f;

    private Coroutine currentFadeCoroutine;
    private Action onCloseCallback;

    public bool IsActive => gameObject.activeSelf;

    void Awake()
    {
        if (panelCanvasGroup == null) panelCanvasGroup = GetComponent<CanvasGroup>();
        if (contentScrollRect == null) contentScrollRect = GetComponentInChildren<ScrollRect>();

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
    }

    void Update()
    {
        if (!IsActive) return;

        // Handle Backspace to close
        if (Keyboard.current != null && Keyboard.current.backspaceKey.wasPressedThisFrame)
        {
            OnCloseClicked();
        }
    }

    public void Show(Action onClose = null)
    {
        onCloseCallback = onClose;
        gameObject.SetActive(true);
        UpdateLanguage();
        
        // Reset scroll position
        if (contentScrollRect != null)
        {
            contentScrollRect.verticalNormalizedPosition = 1f;
        }

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

    public void UpdateLanguage()
    {
        int langIdx = PlayerPrefs.GetInt("Config_Language", 0);

        // Translate the close button
        var closeText = ResolveCloseButtonText();
        if (closeText != null)
        {
            closeText.text = GetSpacingTaggedText(LocalizationManager.Instance.T("close", "CLOSE"), langIdx);
        }

        // Set version entries text explicitly to match the QT version (V1.3Q) dates, headers, and translations
        UpdateEntry("Entry_Version 1.3U", "Version 1.3U", "2026. 06. 23", "changelog_v13", langIdx);
        UpdateEntry("Entry_Version 1.2U", "Version 1.2U", "2026. 05. 10", "changelog_v12", langIdx);
        UpdateEntry("Entry_Version 1.1U", "Version 1.1U", "2026. 03. 27", "changelog_v11", langIdx);
        UpdateEntry("Entry_Version 1.0.1", "Version 1.0.1", "2026. 02. 23", "changelog_v101", langIdx);
        UpdateEntry("Entry_Version 1.0", "Version 1.0", "2026. 02. 22", "changelog_v10", langIdx);
    }

    private void UpdateEntry(string entryName, string versionStr, string dateStr, string changelogKey, int langIdx)
    {
        Transform content = transform.Find("Scroll View/Viewport/Content");
        if (content == null) return;
        Transform entry = content.Find(entryName);
        if (entry == null) return;

        // Find HeaderLine/Text (first child for version, second child for date)
        Transform headerLine = entry.Find("HeaderLine");
        if (headerLine != null && headerLine.childCount >= 2)
        {
            var vText = headerLine.GetChild(0).GetComponent<TextMeshProUGUI>();
            if (vText != null) vText.text = versionStr;

            var dText = headerLine.GetChild(1).GetComponent<TextMeshProUGUI>();
            if (dText != null) dText.text = dateStr;
        }

        // Find the changelog description text child
        Transform descTextTrans = entry.Find("Text");
        if (descTextTrans != null)
        {
            var descText = descTextTrans.GetComponent<TextMeshProUGUI>();
            if (descText != null)
            {
                descText.text = GetSpacingTaggedText(LocalizationManager.Instance.T(changelogKey), langIdx);
            }
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
        return LocalizationManager.Instance.GetSpacingTaggedText(text, lang);
    }

    private string Normalize(string s)
    {
        if (s == null) return "";
        string normalized = s.Replace("\r\n", "\n").Replace("\r", "\n");
        normalized = System.Text.RegularExpressions.Regex.Replace(normalized, "<.*?>", string.Empty);
        return normalized.Trim();
    }
}
