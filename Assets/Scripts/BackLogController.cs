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
    /// </summary>
    public void AddLog(string role, string message)
    {
        if (contentContainer == null || logItemPrefab == null) return;

        GameObject item = Instantiate(logItemPrefab, contentContainer);
        
        // Try to find specific "NameText" and "MessageText" components first
        TextMeshProUGUI nameText = null;
        TextMeshProUGUI messageText = null;
        
        // Search in children by name
        foreach (var t in item.GetComponentsInChildren<TextMeshProUGUI>())
        {
            if (t.name == "NameText") nameText = t;
            else if (t.name == "MessageText") messageText = t;
        }

        // Fallback: If no specifc named components, assume single text or first two
        if (nameText == null && messageText == null)
        {
            var texts = item.GetComponentsInChildren<TextMeshProUGUI>();
            if (texts.Length >= 2) 
            {
                nameText = texts[0];
                messageText = texts[1];
            }
            else if (texts.Length == 1)
            {
                // Single text mode (legacy)
                messageText = texts[0]; 
            }
        }

        string namePrefix = "";
        Color nameColor = Color.white;
        string cleanMessage = message;

        switch (role.ToLower())
        {
            case "user":
            case "me":
                namePrefix = "あなた"; 
                nameColor = userColor;
                break;
            case "assistant":
            case "kurisu":
            case "amadeus":
                namePrefix = "紅莉栖"; 
                nameColor = aiColor;
                break;
            case "system":
                namePrefix = "SYSTEM";
                nameColor = systemColor;
                break;
            default:
                namePrefix = role.ToUpper();
                nameColor = Color.gray;
                break;
        }

        // ─── Dual Text Mode ───
        if (nameText != null && messageText != null)
        {
             Debug.Log($"[BackLog] Setting Dual Text -> Name: {namePrefix}, Msg: {cleanMessage}"); // DEBUG
            nameText.text = namePrefix;
            nameText.color = nameColor;
            messageText.text = cleanMessage; // Ensure not empty
        }
        // ─── Single Text Mode ───
        else if (messageText != null)
        {
            Debug.Log($"[BackLog] Setting Single Text -> {namePrefix}: {cleanMessage}"); // DEBUG
            string colorHex = ColorUtility.ToHtmlStringRGB(nameColor);
            messageText.text = $"<color=#{colorHex}>{namePrefix}:</color> {cleanMessage}";
        }
        else
        {
            Debug.LogError("[BackLog] NO TEXT COMPONENTS FOUND in prefab instance!");
        }

        // ─── Force Layout Rebuild & Visibility ───
        var layout = item.GetComponent<LayoutElement>();
        if (layout == null) layout = item.AddComponent<LayoutElement>();
        layout.minHeight = 30f; // Force minimum height just in case
        
        // Ensure scale is 1
        item.transform.localScale = Vector3.one;

        // Auto-scroll to bottom
        StartCoroutine(ScrollToBottom());
    }

    // Optional reference to get character name if needed
    public TextMeshProUGUI characterNameText;

    private IEnumerator ScrollToBottom()
    {
        yield return new WaitForEndOfFrame();
        if (scrollRect) scrollRect.verticalNormalizedPosition = 0f;
    }
}
