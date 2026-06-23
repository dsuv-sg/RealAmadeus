using System;
using System.Collections;
using UnityEngine;
using UnityEngine.UI;
using UnityEngine.InputSystem;
using TMPro;

public class HelpPanelController : MonoBehaviour
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

        CreateCancelEntry();
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

        // Set help entries text explicitly to match the QT version
        UpdateEntry("Entry_メニュー開閉", "Tab / " + LocalizationManager.Instance.T("help_right_click", "右クリック"), "help_toggle_menu", langIdx);
        UpdateEntry("Entry_保存して閉じる", "Backspace", "help_save_close", langIdx);
        UpdateEntry("Entry_項目選択", "WASD / ↑←↓→", "help_select_item", langIdx);
        UpdateEntry("Entry_選択 / 会話を進める", "Enter", "help_confirm_advance", langIdx);
        UpdateEntry("Entry_会話をキャンセル", "Ctrl+C", "help_cancel_chat", langIdx);
    }

    private void UpdateEntry(string entryName, string headerText, string descKey, int langIdx)
    {
        Transform content = transform.Find("Scroll View/Viewport/Content");
        if (content == null) return;

        Transform entry = null;
        foreach (Transform child in content)
        {
            if (child.name == entryName)
            {
                entry = child;
                break;
            }
        }
        if (entry == null) return;

        // Find HeaderLine/Text (first child)
        Transform headerLine = entry.Find("HeaderLine");
        if (headerLine != null && headerLine.childCount >= 1)
        {
            var hText = headerLine.GetChild(0).GetComponent<TextMeshProUGUI>();
            if (hText != null) hText.text = headerText;
        }

        // Find description Text
        Transform descTextTrans = entry.Find("Text");
        if (descTextTrans != null)
        {
            var descText = descTextTrans.GetComponent<TextMeshProUGUI>();
            if (descText != null)
            {
                descText.text = GetSpacingTaggedText(LocalizationManager.Instance.T(descKey), langIdx);
            }
        }
    }

    private void CreateCancelEntry()
    {
        Transform content = transform.Find("Scroll View/Viewport/Content");
        if (content == null) return;

        // Check if already created (loop check because of possible slash in name)
        bool exists = false;
        foreach (Transform child in content)
        {
            if (child.name == "Entry_会話をキャンセル")
            {
                exists = true;
                break;
            }
        }
        if (exists) return;

        // Find reference to clone (loop check because of slash in name)
        Transform reference = null;
        foreach (Transform child in content)
        {
            if (child.name == "Entry_選択 / 会話を進める")
            {
                reference = child;
                break;
            }
        }
        if (reference == null) return;

        // Clone it
        GameObject clone = Instantiate(reference.gameObject, content);
        clone.name = "Entry_会話をキャンセル";
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
