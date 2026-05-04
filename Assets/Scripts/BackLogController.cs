using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;
using TMPro;

/// <summary>
/// Manages the BackLog UI, displaying conversation history.
/// </summary>
public class BackLogController : MonoBehaviour
{
    private const float EntrySidePadding = 20f;
    private const float EntryVerticalPadding = 5f;
    private const float NameColumnWidth = 115f;
    private const float ColumnSpacing = 150f;

    [Header("UI References")]
    public GameObject backLogPanel;
    public CanvasGroup panelCanvasGroup; // [NEW] Loop
    public Transform contentContainer;
    public GameObject logItemPrefab; 
    public ScrollRect scrollRect;
    public Button closeButton;

    [Header("Settings")]
    public float fadeDuration = 0.2f; // [NEW]
    public Color userColor = new Color(0.4f, 0.8f, 1.0f); 
    public Color aiColor = new Color(1.0f, 0.4f, 0.4f);   
    public Color systemColor = Color.gray;



    private Coroutine currentFadeCoroutine; // [NEW]
    private bool needsScrollToBottom = false;

    public bool IsActive => backLogPanel != null && backLogPanel.activeSelf;

    private void Awake()
    {
        // Auto-get CanvasGroup if missing
        if (backLogPanel != null)
        {
             if (panelCanvasGroup == null) panelCanvasGroup = backLogPanel.GetComponent<CanvasGroup>();
             if (panelCanvasGroup == null) panelCanvasGroup = backLogPanel.AddComponent<CanvasGroup>();
        }

        // Init state: Hiden
        if (panelCanvasGroup != null)
        {
            panelCanvasGroup.alpha = 0f;
            panelCanvasGroup.interactable = false;
            panelCanvasGroup.blocksRaycasts = false;
        }
        if (backLogPanel) backLogPanel.SetActive(false); // Ensure inactive start

        if (closeButton == null)
        {
            Transform btn = FindDeepChild(transform, "Btn_Close");
            if (btn != null) closeButton = btn.GetComponent<Button>();
        }

        if (closeButton) closeButton.onClick.AddListener(Hide);
    }
    
    // ... (AddLog method remains same) ...

    private void Update()
    {
        // Only handle input if visible (alpha high enough)
        if (panelCanvasGroup != null && panelCanvasGroup.alpha > 0.1f)
        {
             if (UnityEngine.InputSystem.Keyboard.current != null && 
                 UnityEngine.InputSystem.Keyboard.current.backspaceKey.wasPressedThisFrame)
             {
                 Hide();
             }
        }
    }

    // ... (characterNameText, ScrollToBottom remain same) ...

    public void Show()
    {
        if (backLogPanel)
        {
            backLogPanel.SetActive(true);
            UpdateLanguage();
            if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
            currentFadeCoroutine = StartCoroutine(FadeCanvas(0f, 1f));
            RecalculateAllLogEntryHeights();
            // Rebuild layout and scroll (handles deferred entries added while inactive)
            if (contentContainer != null)
                LayoutRebuilder.ForceRebuildLayoutImmediate(contentContainer as RectTransform);
            needsScrollToBottom = false;
            StartCoroutine(ScrollToBottom());
        }
    }

    public void Hide()
    {
        if (backLogPanel)
        {
            if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
            currentFadeCoroutine = StartCoroutine(FadeCanvas(panelCanvasGroup ? panelCanvasGroup.alpha : 1f, 0f, true));
        }
    }

    public void Toggle()
    {
        if (backLogPanel)
        {
             if (panelCanvasGroup != null && panelCanvasGroup.alpha > 0.5f) Hide();
             else Show();
        }
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
        if (disableOnFinish) backLogPanel.SetActive(false);
    }


    /// <summary>
    /// Adds a new entry to the backlog.
    /// Creates a simple text item programmatically for reliable layout.
    /// </summary>
    public void AddLog(string role, string message)
    {
        if (contentContainer == null) return;
        if (string.IsNullOrWhiteSpace(message)) return;

        // Strip emotion tags like [HAPPY], [ANGRY] etc from message
        string cleanMessage = message.Trim();
        if (cleanMessage.StartsWith("["))
        {
            int closeBracket = cleanMessage.IndexOf(']');
            if (closeBracket > 0)
            {
                cleanMessage = cleanMessage.Substring(closeBracket + 1).Trim();
            }
        }
        if (string.IsNullOrWhiteSpace(cleanMessage)) return;

        // Determine display info
        int langIdx = PlayerPrefs.GetInt("Config_Language", 0);
        string namePrefix = GetNameForRole(role, langIdx);
        Color nameColor;
        switch (role.ToLower())
        {
            case "user": case "me":
                nameColor = userColor; break;
            case "assistant": case "kurisu": case "amadeus":
                nameColor = aiColor; break;
            case "system":
                nameColor = systemColor; break;
            default:
                nameColor = Color.gray; break;
        }

        // Create item from scratch for reliable layout
        GameObject item = new GameObject("LogEntry_" + role.ToLower(), typeof(RectTransform), typeof(CanvasRenderer));
        item.transform.SetParent(contentContainer, false);

        // RectTransform: stretch horizontally, auto height
        RectTransform rt = item.GetComponent<RectTransform>();
        rt.anchorMin = new Vector2(0, 0);
        rt.anchorMax = new Vector2(1, 0);
        rt.pivot = new Vector2(0.5f, 0);
        rt.sizeDelta = new Vector2(0, 0);

        // Background image (subtle)
        var bg = item.AddComponent<Image>();
        bg.color = new Color(1f, 1f, 1f, 0f);

        // ContentSizeFitter to auto-size height based on text
        var csf = item.AddComponent<ContentSizeFitter>();
        csf.horizontalFit = ContentSizeFitter.FitMode.Unconstrained;
        csf.verticalFit = ContentSizeFitter.FitMode.PreferredSize;

        var hlg = item.AddComponent<HorizontalLayoutGroup>();
        hlg.padding = new RectOffset((int)EntrySidePadding, (int)EntrySidePadding, (int)EntryVerticalPadding, (int)EntryVerticalPadding);
        hlg.spacing = ColumnSpacing;
        hlg.childControlWidth = true;
        hlg.childControlHeight = true;
        hlg.childForceExpandWidth = false;
        hlg.childForceExpandHeight = false;

        // Name column
        var nameObj = new GameObject("NameText", typeof(RectTransform), typeof(CanvasRenderer));
        nameObj.transform.SetParent(item.transform, false);
        var nameLE = nameObj.AddComponent<LayoutElement>();
        nameLE.minWidth = NameColumnWidth;
        nameLE.preferredWidth = NameColumnWidth;
        nameLE.flexibleWidth = 0f;

        var nameTMP = nameObj.AddComponent<TextMeshProUGUI>();
        nameTMP.text = $"<b>{namePrefix}</b>";
        nameTMP.fontSize = 26;
        nameTMP.color = nameColor;
        nameTMP.alignment = TextAlignmentOptions.TopLeft;
        nameTMP.enableWordWrapping = false;
        nameTMP.overflowMode = TextOverflowModes.Overflow;
        nameTMP.raycastTarget = false;

        // Message column
        var messageObj = new GameObject("MessageText", typeof(RectTransform), typeof(CanvasRenderer));
        messageObj.transform.SetParent(item.transform, false);
        var messageLE = messageObj.AddComponent<LayoutElement>();
        messageLE.minWidth = 100f;
        messageLE.flexibleWidth = 1f;

        var messageTMP = messageObj.AddComponent<TextMeshProUGUI>();
        messageTMP.text = cleanMessage;
        messageTMP.fontSize = 26;
        messageTMP.color = Color.white;
        messageTMP.alignment = TextAlignmentOptions.TopLeft;
        messageTMP.enableWordWrapping = true;
        messageTMP.overflowMode = TextOverflowModes.Overflow;
        messageTMP.raycastTarget = false;

        // Try to use the same font as the prefab
        if (logItemPrefab != null)
        {
            var prefabTMP = logItemPrefab.GetComponentInChildren<TextMeshProUGUI>();
            if (prefabTMP != null && prefabTMP.font != null)
            {
                nameTMP.font = prefabTMP.font;
                nameTMP.fontSharedMaterial = prefabTMP.fontSharedMaterial;
                messageTMP.font = prefabTMP.font;
                messageTMP.fontSharedMaterial = prefabTMP.fontSharedMaterial;
            }
        }

        // LayoutElement for proper height reporting
        var le = item.AddComponent<LayoutElement>();
        le.minHeight = 40f;
        le.flexibleWidth = 1f;

        UpdateLogEntryHeight(messageTMP, le);

        item.transform.localScale = Vector3.one;

        // Auto-scroll to bottom (only if panel is active, otherwise Show() will handle it)
        if (backLogPanel != null && backLogPanel.activeInHierarchy)
        {
            LayoutRebuilder.ForceRebuildLayoutImmediate(contentContainer as RectTransform);
            StartCoroutine(ScrollToBottom());
        }
        else
        {
            needsScrollToBottom = true;
        }
    }

    // Optional reference to get character name if needed
    public TextMeshProUGUI characterNameText;

    private IEnumerator ScrollToBottom()
    {
        yield return new WaitForEndOfFrame();
        if (scrollRect) scrollRect.verticalNormalizedPosition = 0f;
    }

    public void ClearLogs()
    {
        if (contentContainer == null) return;
        foreach (Transform child in contentContainer)
        {
            Destroy(child.gameObject);
        }
        needsScrollToBottom = true;
    }

    private void RecalculateAllLogEntryHeights()
    {
        if (contentContainer == null) return;

        foreach (Transform child in contentContainer)
        {
            var messageTransform = child.Find("MessageText");
            if (messageTransform == null) continue;

            var tmp = messageTransform.GetComponent<TextMeshProUGUI>();
            if (tmp == null) continue;

            var le = child.GetComponent<LayoutElement>();
            if (le == null)
            {
                le = child.gameObject.AddComponent<LayoutElement>();
                le.minHeight = 40f;
                le.flexibleWidth = 1f;
            }

            UpdateLogEntryHeight(tmp, le);
        }
    }

    private void UpdateLogEntryHeight(TextMeshProUGUI tmp, LayoutElement le)
    {
        if (tmp == null || le == null) return;

        const float verticalPadding = EntryVerticalPadding * 2f;

        float availableWidth = 600f;
        RectTransform containerRect = contentContainer as RectTransform;
        if (containerRect != null && containerRect.rect.width > 0f)
        {
            float innerWidth = containerRect.rect.width - (EntrySidePadding * 2f);
            availableWidth = Mathf.Max(1f, innerWidth - NameColumnWidth - ColumnSpacing);
        }

        tmp.ForceMeshUpdate();
        Vector2 preferred = tmp.GetPreferredValues(tmp.text, availableWidth, 0f);
        le.preferredHeight = Mathf.Max(le.minHeight, preferred.y + verticalPadding);
    }

    public void UpdateLanguage()
    {
        if (contentContainer == null) return;

        int langIdx = PlayerPrefs.GetInt("Config_Language", 0);

        foreach (Transform child in contentContainer)
        {
            string role = child.name.StartsWith("LogEntry_") ? child.name.Substring(9) : "";
            if (string.IsNullOrEmpty(role)) continue;

            Transform nameTf = child.Find("NameText");
            if (nameTf == null) continue;

            var tmp = nameTf.GetComponent<TextMeshProUGUI>();
            if (tmp == null) continue;

            string namePrefix = GetNameForRole(role, langIdx);
            tmp.text = $"<b>{GetSpacingTaggedText(namePrefix, langIdx)}</b>";
        }

        var closeButtonText = ResolveCloseButtonText();
        if (closeButtonText != null)
        {
            string Close_JA = "閉じる", Close_EN = "Close", Close_ZH = "关闭", Close_KO = "닫기", Close_ES = "Cerrar", Close_FR = "Fermer", Close_DE = "Schließen", Close_RU = "Закрыть";
            if (langIdx == 1) closeButtonText.text = Close_EN;
            else if (langIdx == 2) closeButtonText.text = Close_ZH;
            else if (langIdx == 3) closeButtonText.text = Close_KO;
            else if (langIdx == 4) closeButtonText.text = Close_ES;
            else if (langIdx == 5) closeButtonText.text = Close_FR;
            else if (langIdx == 6) closeButtonText.text = Close_DE;
            else if (langIdx == 7) closeButtonText.text = GetSpacingTaggedText(Close_RU, langIdx);
            else closeButtonText.text = Close_JA;
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

    private string GetNameForRole(string role, int langIdx)
    {
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

        switch (role.ToLower())
        {
            case "user": case "me":
                return getTr("あなた", "You", "你", "당신", "Tú", "Vous", "Du", "Вы");
            case "assistant": case "kurisu": case "amadeus":
                return getTr("アマデウス紅莉栖", "Amadeus Kurisu", "阿玛迪斯·红莉栖", "아마데우스 쿠리스", "Amadeus Kurisu", "Amadeus Kurisu", "Amadeus Kurisu", "Амадеус Курису");
            default:
                return role.ToUpper();
        }
    }
}
