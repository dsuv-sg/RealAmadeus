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
            if (currentFadeCoroutine != null) StopCoroutine(currentFadeCoroutine);
            currentFadeCoroutine = StartCoroutine(FadeCanvas(0f, 1f));
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
        string namePrefix;
        Color nameColor;
        switch (role.ToLower())
        {
            case "user": case "me":
                namePrefix = "あなた"; nameColor = userColor; break;
            case "assistant": case "kurisu": case "amadeus":
                namePrefix = "紅莉栖"; nameColor = aiColor; break;
            case "system":
                namePrefix = "SYSTEM"; nameColor = systemColor; break;
            default:
                namePrefix = role.ToUpper(); nameColor = Color.gray; break;
        }

        // Create item from scratch for reliable layout
        GameObject item = new GameObject("LogEntry", typeof(RectTransform), typeof(CanvasRenderer));
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

        // Single TMP text with rich text for name coloring
        var textObj = new GameObject("Text", typeof(RectTransform), typeof(CanvasRenderer));
        textObj.transform.SetParent(item.transform, false);

        RectTransform textRT = textObj.GetComponent<RectTransform>();
        textRT.anchorMin = Vector2.zero;
        textRT.anchorMax = Vector2.one;
        textRT.offsetMin = new Vector2(20, 5);
        textRT.offsetMax = new Vector2(-20, -5);

        var tmp = textObj.AddComponent<TextMeshProUGUI>();
        string hexColor = ColorUtility.ToHtmlStringRGB(nameColor);
        tmp.text = $"<color=#{hexColor}><b>{namePrefix}</b></color>　{cleanMessage}";
        tmp.fontSize = 26;
        tmp.color = Color.white;
        tmp.enableWordWrapping = true;
        tmp.overflowMode = TextOverflowModes.Overflow;
        tmp.raycastTarget = false;

        // Try to use the same font as the prefab
        if (logItemPrefab != null)
        {
            var prefabTMP = logItemPrefab.GetComponentInChildren<TextMeshProUGUI>();
            if (prefabTMP != null && prefabTMP.font != null)
            {
                tmp.font = prefabTMP.font;
                tmp.fontSharedMaterial = prefabTMP.fontSharedMaterial;
            }
        }

        // LayoutElement for proper height reporting
        var le = item.AddComponent<LayoutElement>();
        le.minHeight = 40f;
        le.flexibleWidth = 1f;

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
}
